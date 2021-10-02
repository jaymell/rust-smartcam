use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::{debug, error, trace};
use opencv::{
    core,
    core::Point,
    core::Scalar,
    core::BORDER_CONSTANT,
    core::no_array,
    highgui,
    imgproc,
    imgproc::LINE_AA,
    imgproc::CHAIN_APPROX_SIMPLE,
    imgproc::RETR_TREE,
    imgproc::THRESH_BINARY,
    prelude::*,
    types::VectorOfMat,
};
use std::sync::mpsc::Receiver;
use std::error::Error;

use crate::frame::{Frame, VideoFrame};
use crate::video_writer::VideoWriter;
use crate::config::load_config;

pub struct MotionDetector {
    receiver: Receiver<Frame>,
    video_writer: Option<VideoWriter>,
    in_motion: bool,
    in_motion_window: bool,
    last_motion_time: DateTime<Utc>,
    min_threshold_size: i32,
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
    pub fn new(receiver: Receiver<Frame>) -> Self {
        let cfg = load_config(None);
        Self {
            receiver,
            video_writer: None,
            in_motion: false,
            in_motion_window: false,
            last_motion_time: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            min_threshold_size: cfg.motion.min_threshold_size,
        }
    }

    pub fn start(&mut self) -> () {
        // Dump first images:
        for _ in 1..20 {
            self.receiver.recv().unwrap();
        }

        let mut previous = self.receiver.recv().unwrap().downsample().unwrap();
        let window = "motion detection";
        debug!("Opening motion detection window");
        highgui::named_window(window, highgui::WINDOW_AUTOSIZE).unwrap();

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

            let mut contour_frame = org_frame.clone();
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

                    // FIXME -- this doesn't need to be done in foor loop:
                    match imgproc::draw_contours(
                        contour_frame.img_mut(),
                        &c,
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

                    if !self.in_motion {
                        self.send_frame(VideoFrame {
                            frame: contour_frame.clone(),
                            is_start: true,
                            is_end: false,
                        });
                    }
                    self.in_motion = true;
                    self.in_motion_window = true;
                    self.last_motion_time = frame.time();
                    debug!("Motion detected at {:?}", self.last_motion_time);

                    break;
                }
            }

            if self.in_motion_window {
                if !check_in_motion_window(frame.time(), self.last_motion_time) {
                    debug!("Motion window closing.");
                    self.in_motion_window = false;
                    self.send_frame(VideoFrame {
                        frame: contour_frame.clone(),
                        is_start: false,
                        is_end: true,
                    });
                    self.video_writer = None;
                } else {
                    self.send_frame(VideoFrame {
                        frame: contour_frame.clone(),
                        is_start: false,
                        is_end: false,
                    });
                }
            }

            // if let Err(error) = highgui::imshow(window, &dilated) {
            if let Err(error) = highgui::imshow(window, &contour_frame.img()) {
                error!("highgui::imshow failed: {:?}", error);
                continue;
            }

            if let Err(error) = highgui::wait_key(1) {
                error!("highgui::wait_key failed: {:?}", error);
                continue;
            }

            previous = frame;
        }
    }

    fn send_frame(&mut self, frame: VideoFrame) -> () {
        match &self.video_writer {
            Some(v) => {
                v.send_frame(frame);
            }
            None => {
                let v = VideoWriter::new();
                v.send_frame(frame);
                self.video_writer = Some(v);
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
