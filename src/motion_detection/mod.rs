use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::{debug, error, trace, warn};
use opencv::{
    core,
    core::no_array,
    core::Point,
    core::Scalar,
    core::BORDER_CONSTANT,
    highgui, imgproc,
    imgproc::{bounding_rect, rectangle, CHAIN_APPROX_SIMPLE, LINE_AA, RETR_TREE, THRESH_BINARY},
    prelude::*,
    types::VectorOfMat,
};
use std::error::Error;
use std::sync::{mpsc::Receiver, mpsc::Sender, Arc};

use crate::config::load_config;
use crate::config::CameraConfig;
use crate::frame::{Frame, VideoFrame};
use crate::video;

pub struct MotionDetector {
    receiver: Receiver<Arc<Frame>>,
    video_tx: Option<Sender<VideoFrame>>,
    in_motion: bool,
    in_motion_window: bool,
    last_motion_time: DateTime<Utc>,
    min_threshold_size: i32,
    camera: Arc<CameraConfig>,
    draw_contours: bool,
    draw_rectangles: bool,
}

fn absdiff(img1: &Mat, img2: &Mat) -> Result<Mat, Box<dyn Error>> {
    let mut delta = Mat::default();
    core::absdiff(img1, img2, &mut delta)?;
    Ok(delta)
}

fn threshold(img: &Mat) -> Result<Mat, Box<dyn Error>> {
    let mut thresh = Mat::default();
    imgproc::threshold(img, &mut thresh, 25.0, 255.0, THRESH_BINARY)?;
    Ok(thresh)
}

fn dilate(img: &Mat) -> Result<Mat, Box<dyn Error>> {
    let mut dilated = Mat::default();
    imgproc::dilate(
        img,
        &mut dilated,
        &Mat::default(),
        Point::new(1, 1),
        1,
        BORDER_CONSTANT,
        imgproc::morphology_default_border_value().unwrap(),
    )?;
    Ok(dilated)
}

fn find_contours(img: &Mat) -> Result<VectorOfMat, Box<dyn Error>> {
    let mut contours = VectorOfMat::new();
    imgproc::find_contours(
        img,
        &mut contours,
        RETR_TREE,
        CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    Ok(contours)
}

impl MotionDetector {
    pub fn new(camera: Arc<CameraConfig>, receiver: Receiver<Arc<Frame>>) -> Self {
        let cfg = load_config(None);
        Self {
            receiver,
            video_tx: None,
            in_motion: false,
            in_motion_window: false,
            last_motion_time: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            min_threshold_size: cfg.motion.min_threshold_size,
            camera,
            draw_contours: cfg.motion.draw_contours.unwrap_or_default(),
            draw_rectangles: cfg.motion.draw_rectangles.unwrap_or_default(),
        }
    }

    pub fn start(&mut self) -> () {
        debug!("Starting motion detector");

        // Dump first images:
        for _ in 1..20 {
            self.receiver.recv().unwrap();
        }

        let mut previous = self.receiver.recv().unwrap().downsample().unwrap();

        loop {
            let org_frame = match self.receiver.recv() {
                Ok(frame) => frame,
                Err(error) => {
                    error!("Failed to receieve frame: {:?}", error);
                    continue;
                }
            };
            let frame = match org_frame.downsample() {
                Ok(downsampled) => downsampled,
                Err(error) => {
                    error!("Failed to downsample frame: {:?}", error);
                    continue;
                }
            };

            let delta = absdiff(&previous.img(), &frame.img()).unwrap();
            let thresh = threshold(&delta).unwrap();
            let dilated = dilate(&thresh).unwrap();
            let contours = find_contours(&dilated);
            if let Err(e) = contours {
                error!("Failed to find contours: {:?}", e);
                continue;
            }
            let contours = contours.unwrap();

            let mut contour_frame = Arc::new((*org_frame).clone());

            let mut frame_sent = false;
            for c in contours.iter() {
                trace!("Contours: {:?}", c);
                let area = match imgproc::contour_area(&c, false) {
                    Ok(a) => a,
                    Err(error) => {
                        error!("Failed to get contour area: {:?}", error);
                        continue;
                    }
                };

                if area as i32 >= self.min_threshold_size {
                    // Motion detected:
                    if self.draw_contours {
                        match Arc::get_mut(&mut contour_frame) {
                            Some(f) => draw_contours(f, &contours),
                            None => warn!("Unable to get contour_frame mutable ref"),
                        }
                    }
                    if self.draw_rectangles {
                        match Arc::get_mut(&mut contour_frame) {
                            Some(f) => draw_rectangles(f, &contours),
                            None => warn!("Unable to get contour_frame mutable ref"),
                        }
                    }
                    // send first frame:
                    if !self.in_motion {
                        self.send_frame(VideoFrame {
                            frame: Arc::clone(&contour_frame),
                            is_start: true,
                            is_end: false,
                        });
                        frame_sent = true;
                    }
                    self.in_motion = true;
                    self.in_motion_window = true;
                    self.last_motion_time = frame.time();

                    debug!("Motion detected at {:?}", self.last_motion_time);

                    break;
                }
            }

            if self.in_motion_window && !frame_sent {
                if !check_in_motion_window(frame.time(), self.last_motion_time) {
                    debug!("Motion window closing.");
                    self.in_motion_window = false;
                    self.send_frame(VideoFrame {
                        frame: Arc::clone(&contour_frame),
                        is_start: false,
                        is_end: true,
                    });
                    self.video_tx = None;
                } else {
                    self.send_frame(VideoFrame {
                        frame: Arc::clone(&contour_frame),
                        is_start: false,
                        is_end: false,
                    });
                }
                frame_sent = true;
            }

            previous = frame;
        }
    }

    fn send_frame(&mut self, frame: VideoFrame) -> () {
        match &self.video_tx {
            Some(v) => {
                v.send(frame).unwrap();
            }
            None => {
                let f = &frame.frame;
                let v = video::start_video_writer(
                    Arc::clone(&self.camera),
                    f.time(),
                    f.width(),
                    f.height(),
                );
                v.send(frame).unwrap();
                self.video_tx = Some(v);
            }
        };
    }
}

fn check_in_motion_window(current_time: DateTime<Utc>, last_motion_time: DateTime<Utc>) -> bool {
    let min_motion_capture_time: Duration = Duration::seconds(10);
    if (current_time - min_motion_capture_time) >= last_motion_time {
        false
    } else {
        true
    }
}

fn draw_contours(frame: &mut Frame, contours: &VectorOfMat) {
    match imgproc::draw_contours(
        frame.img_mut(),
        contours,
        -1,
        Scalar::new(0.0, 0.0, 255.0, 0.0),
        4,
        LINE_AA,
        &no_array().unwrap(),
        1,
        Point::new(0, 0),
    ) {
        Ok(_) => (),
        Err(e) => error!("Drawing contours failed: {}", e),
    }
}

fn draw_rectangles(frame: &mut Frame, contours: &VectorOfMat) {
    trace!("draw_rectangles contours: {:?}", contours);

    contours.iter().for_each(|ctr| {
        let rect = match bounding_rect(&ctr) {
            Ok(rect) => rect,
            Err(e) => {
                error!("Failed to get bounding rectangle: {}", e);
                return;
            }
        };

        match rectangle(
            frame.img_mut(),
            rect,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
            4,
            LINE_AA,
            0,
        ) {
            Ok(_) => (),
            Err(e) => error!("Failed to draw bounding rectangle: {}", e),
        }
    });
}
