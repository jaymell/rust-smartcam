use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::{debug, error, trace};
use opencv::{
    core::absdiff, core::Point, core::BORDER_CONSTANT, highgui, imgproc::contour_area,
    imgproc::dilate, imgproc::find_contours, imgproc::morphology_default_border_value,
    imgproc::threshold, imgproc::CHAIN_APPROX_SIMPLE, imgproc::RETR_TREE, imgproc::THRESH_BINARY,
    prelude::*, types::VectorOfMat, Result,
};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::frame::{Frame, VideoFrame};
use crate::video_writer::VideoWriter;

const MIN_CONTOUR_AREA: u16 = 200;

pub struct MotionDetector {
    receiver: Receiver<Frame>,
    video_writer: Option<VideoWriter>,
    in_motion: bool,
    in_motion_window: bool,
    last_motion_time: DateTime<Utc>,
}

impl MotionDetector {
    pub fn new(receiver: Receiver<Frame>) -> Self {
        Self {
            receiver,
            video_writer: None,
            in_motion: false,
            in_motion_window: false,
            last_motion_time: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let mut previous = self.receiver.recv().unwrap().downsample()?;
        let window = "motion detection";
        debug!("opening motion detection window");
        highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
        debug!("opening motion detection window DONE");

        loop {
            let frame = match self.receiver.recv() {
                Ok(frame) => match frame.downsample() {
                    Ok(downsampled) => downsampled,
                    Err(error) => {
                        error!("Failed to downsample frame: {:?}", error);
                        continue;
                    }
                },
                Err(error) => {
                    error!("Failed to receieve frame: {:?}", error);
                    continue;
                }
            };

            let mut delta = Mat::default();
            absdiff(&previous.get_img(), &frame.get_img(), &mut delta);

            let mut thresh = Mat::default();
            threshold(&delta, &mut thresh, 25.0, 255.0, THRESH_BINARY);

            let mut dilated = Mat::default();

            // TODO: No idea if this needs to be called every time or can just be called once:
            let bv = match morphology_default_border_value() {
                Ok(v) => v,
                Err(error) => {
                    error!("morphology_default_border_value failed: {:?}", error);
                    continue;
                }
            };
            dilate(
                &thresh,
                &mut dilated,
                &Mat::default(),
                Point::new(1, 1),
                1,
                BORDER_CONSTANT,
                bv,
            );

            let mut contours = VectorOfMat::new();

            if let Err(error) = find_contours(
                &dilated,
                &mut contours,
                RETR_TREE,
                CHAIN_APPROX_SIMPLE,
                Point::new(0, 0),
            ) {
                error!("Failed to find contours: {:?}", error);
                continue;
            }

            for c in contours.iter() {
                trace!("Contours: {:?}", c);
                let area = match contour_area(&c, false) {
                    Ok(a) => a,
                    Err(error) => {
                        error!("Failed to get contour area: {:?}", error);
                        continue;
                    }
                };

                if area as u16 >= MIN_CONTOUR_AREA {
                    // Motion detected:
                    if !self.in_motion {
                        self.send_frame(VideoFrame {
                            frame: frame.clone(),
                            is_start: true,
                            is_end: false,
                        });
                    }
                    self.in_motion = true;
                    self.in_motion_window = true;
                    self.last_motion_time = frame.time;
                    debug!("Motion detected at {:?}", self.last_motion_time);
                    break;
                }
            }

            if !self.in_motion && self.in_motion_window {
                if !check_in_motion_window(frame.time, self.last_motion_time) {
                    debug!("Motion window closing.");
                    self.in_motion_window = false;
                    self.send_frame(VideoFrame {
                        frame: frame.clone(),
                        is_start: false,
                        is_end: true,
                    });
                } else {
                    self.send_frame(VideoFrame {
                        frame: frame.clone(),
                        is_start: false,
                        is_end: false,
                    });
                }
            }

            if let Err(error) = highgui::imshow(window, &dilated) {
                error!("highgui::imshow failed: {:?}", error);
                continue;
            }

            if let Err(error) = highgui::wait_key(1) {
                error!("highgui::wait_key failed: {:?}", error);
                continue;
            }

            previous = frame;
        }

        Ok(())
    }

    fn send_frame(&mut self, frame: VideoFrame) -> () {
        match self.video_writer {
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
    let min_motion_capture_time: Duration = Duration::seconds(20);
    if (current_time - min_motion_capture_time) >= last_motion_time {
        false
    } else {
        true
    }
}
