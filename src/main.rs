mod config;
mod frame;
mod frame_reader;
mod logger;
mod motion_detection;
mod repository;
mod upload;
mod video;
mod web;

use self::motion_detection::MotionDetector;
use crate::frame::Frame;
pub(crate) use config::FileSourceType;
use log::debug;
use std::process;
use std::sync::{mpsc::channel, Arc};
use std::thread;
use std::thread::JoinHandle;
use tokio::sync::mpsc::{channel as async_channel, Receiver as AsyncReceiver};

#[macro_use]
extern crate rocket;
fn main() -> () {
    logger::init().unwrap();

    let config = config::load_config(None);
    debug!("Config: {:?}", config);

    let display_enabled = config.display.enabled.unwrap_or(true);
    let (mut threads, web_rx_vec) = launch(config.cameras.clone(), display_enabled);

    let (tx, rx) = channel();
    let ctrlc_thread = thread::spawn(move || -> () {
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
            .expect("Error setting Ctrl-C handler");
        rx.recv().expect("Could not receive from channel.");
        process::exit(0);
    });
    threads.push(ctrlc_thread);

    if display_enabled {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(web::start(web_rx_vec.unwrap(), config.cameras.clone()));
    }

    threads.into_iter().for_each(|t: JoinHandle<()>| {
        t.join().unwrap();
    });
}

fn launch(
    cameras: Vec<config::CameraConfig>,
    display_enabled: bool,
) -> (Vec<JoinHandle<()>>, Option<Vec<AsyncReceiver<Arc<Frame>>>>) {
    let mut threads = Vec::new();
    let mut web_rx_vec = if display_enabled {
        Some(Vec::new())
    } else {
        None
    };

    cameras
        .into_iter()
        .for_each(|camera: config::CameraConfig| {
            let camera = Arc::new(camera);
            let (motion_tx, motion_rx) = channel::<Arc<Frame>>();
            let (web_tx, web_rx) = if display_enabled {
                let (t, r) = async_channel::<Arc<Frame>>(1000);
                (Some(t), Some(r))
            } else {
                (None, None)
            };

            let tx_vec = vec![motion_tx];

            let cam = Arc::clone(&camera);
            let frame_reader_thread = thread::spawn(move || -> () {
                frame_reader::start_frame_reader(cam, tx_vec, web_tx)
                    .expect("Failed to start frame reader");
            });

            let cam = Arc::clone(&camera);
            let motion_detector_thread = thread::spawn(move || -> () {
                let mut md = MotionDetector::new(cam, motion_rx);
                md.start();
            });

            threads.push(frame_reader_thread);
            threads.push(motion_detector_thread);
            if let Some(v) = &mut web_rx_vec {
                v.push(web_rx.unwrap());
            }
        });

    (threads, web_rx_vec)
}
