use std::{sync::mpsc, thread};

mod config;
mod frame;
mod frame_reader;
mod frame_viewer;
mod logger;
mod motion_detector;
mod video_writer;
mod upload;

use self::motion_detector::MotionDetector;
use crate::frame::Frame;

fn main() -> () {
    logger::init().unwrap();

    let config = config::load_config(None);

    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();

    let frame_reader_thread = thread::spawn(move || -> () {
        match config.cameras[0].camera_type.as_str() {
            "rtsp" => frame_reader::start_rtsp(vec![frame_tx], &(config.cameras[0]).source),
            _ => frame_reader::start_v4l(vec![frame_tx]),
        };
    });

    let motion_detector_thread = thread::spawn(move || -> () {
        let mut md = MotionDetector::new(frame_rx);
        md.start();
    });

    frame_reader_thread.join().unwrap();
    motion_detector_thread.join().unwrap();
}
