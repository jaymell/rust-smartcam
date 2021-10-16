mod config;
mod frame;
mod frame_reader;
mod frame_viewer;
mod logger;
mod motion_detector;
mod upload;
mod video_writer;

use self::motion_detector::MotionDetector;
use crate::frame::Frame;
use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use glib::{clone, Bytes, Continue, MainContext, Receiver, PRIORITY_DEFAULT};
use gtk::prelude::*;
use gtk::{ Grid, FlowBox, SelectionMode, Orientation };
use log::{debug, info};
use opencv::prelude::MatTraitManual;
use std::sync::{mpsc::channel, mpsc::Sender, Arc, Mutex};
use std::thread::JoinHandle;
use std::{cell::Cell, rc::Rc, thread, time};

const MAX_IMAGE_WIDTH: u32 = 400;
const MAX_IMAGE_HEIGHT: u32 = 400;

fn crop_scale(pixbuf: &Pixbuf, cur_w: Option<u32>, cur_h: Option<u32>, num_items: usize) -> Option<Pixbuf> {
    // crop to square
    let w = pixbuf.width();
    let h = pixbuf.height();
    if h == w {
        return pixbuf.scale_simple(
                     MAX_IMAGE_WIDTH as _,
                     MAX_IMAGE_HEIGHT as _,
                     InterpType::Bilinear
               );
    }
    let (x, y, dim) = if w > h {
        ((w - h) / 2, 0, h)
    } else {
        (0, (h - w) / 2, w)
    };
    // resize
    pixbuf.new_subpixbuf(x, y, dim, dim)
        .unwrap()
        .scale_simple(MAX_IMAGE_WIDTH as _, MAX_IMAGE_HEIGHT as _, InterpType::Bilinear)
}

fn build_ui(application: &gtk::Application, window_rx: Vec<(Receiver<Arc<Frame>>, String)>) {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .title("smartcam")
        .border_width(10)
        .window_position(gtk::WindowPosition::Center)
        .default_width(1)
        .default_height(1)
        .build();

    let flow = FlowBox::new();
    flow.set_column_spacing(10);
    flow.set_selection_mode(SelectionMode::None);
    flow.set_orientation(Orientation::Horizontal);
    flow.set_min_children_per_line(2);
    let cur_w: Option<u32> = None;
    let cur_h: Option<u32> = None;
    let num_items = window_rx.len();
    window_rx.into_iter().for_each(|(rx, camera_label)| {

        let b = gtk::Box::new(gtk::Orientation::Vertical, 5);
        let image_widget = gtk::Image::new();
        let label = gtk::Label::new(None);
        label.set_text(&camera_label);
        image_widget.set_margin_end(10);
        b.pack_start(&label, true, true, 0);
        b.pack_start(&image_widget, true, true, 0);
        let flow_child = gtk::FlowBoxChildBuilder::new()
            .child(&b)
            .build();
        debug!("Attaching {} to flowbox", label);
        flow.add(&flow_child);

        rx.attach(None, move |f: Arc<Frame>| {
            let buf = f
                .colorspace()
                .convert_buf(f.buf().unwrap(), crate::frame::Colorspace::RGB);
            let pixbuf = Pixbuf::from_bytes(
                &Bytes::from_owned(buf),
                Colorspace::Rgb,
                false,
                8,
                f.width() as _,
                f.height() as _,
                f.width() as i32 * 3,
            );
            let scaled = crop_scale(&pixbuf, cur_w, cur_h, num_items).unwrap();
            image_widget.set_from_pixbuf(Some(&scaled));
            Continue(true)
        });

    });

    // window.connect_configure_event(|_, y| {
    //     let (h, w) = y.size();
    //     debug!("Current window size: {}x{}", h, w);
    //     false
    // });


    window.add(&flow);
    window.show_all();
}

fn main() -> () {
    logger::init().unwrap();

    let config = config::load_config(None);

    let display_enabled = match config.display.enabled {
        Some(e) => e,
        // default enabled if not specified:
        None => true,
    };

    launch(config.cameras.clone(), display_enabled);
}

fn launch(cameras: Vec<config::CameraConfig>, display_enabled: bool) {
    let mut window_rx = Vec::new();
    let mut threads = Vec::new();

    cameras
        .into_iter()
        .for_each(|camera: config::CameraConfig| {
            let (frame_tx, frame_rx) = channel::<Arc<Frame>>();
            let mut window_tx: Option<glib::Sender<Arc<Frame>>> = None;

            if display_enabled {
                let (tx, rx) = MainContext::channel::<Arc<Frame>>(PRIORITY_DEFAULT);
                window_tx = Some(tx);
                window_rx.push(Cell::new(Some((rx, camera.label.clone()))));
            } else {
                info!("Display disabled.");
            }

            let frame_reader_thread = thread::spawn(move || -> () {
                match camera.camera_type.as_str() {
                    "rtsp" => frame_reader::start_rtsp(vec![frame_tx], camera.source.as_deref(), window_tx),
                    _ => frame_reader::start_v4l(vec![frame_tx], camera.source.as_deref(), window_tx),
                };
            });

            let motion_detector_thread = thread::spawn(move || -> () {
                let mut md = MotionDetector::new(frame_rx);
                md.start();
            });

            threads.push(frame_reader_thread);
            threads.push(motion_detector_thread);
        });

    let mut application: Option<gtk::Application> = None;

    if display_enabled {
        application = Some(gtk::Application::new(
            Some("com.github.jaymell.rust-smartcam"),
            Default::default(),
        ));
    }
    if let Some(application) = application {
        let window_rx: Cell<Option<Vec<Cell<Option<(Receiver<Arc<Frame>>, String)>>>>> =
            Cell::new(Some(window_rx));
        application.connect_activate(move |app: &gtk::Application| {
            build_ui(
                app,
                window_rx
                    .take()
                    .expect("Repeated call of connect_activate closure")
                    .iter()
                    .map(|x| -> (Receiver<Arc<Frame>>, String) {
                        x.take().expect("Repeated call of connect_activate closure")
                    })
                    .collect(),
            );
        });

        application.run();
    }

    threads.into_iter().for_each(|t: JoinHandle<()>| {
        t.join().unwrap();
    });
}
