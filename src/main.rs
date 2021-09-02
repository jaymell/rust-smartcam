use std::{sync::mpsc, thread};

mod frame;
mod frame_reader;
mod frame_splitter;
mod frame_viewer;
mod logger;
mod motion_detector;
use self::motion_detector::MotionDetector;
mod uploader;
mod video_writer;

use crate::frame::Frame;

fn main() -> () {
    logger::init().unwrap();

    let (frame_tx, frame_rx) = mpsc::channel::<Frame>();

    let frame_reader_thread = thread::spawn(move || -> () {
        frame_reader::start(frame_tx);
    });

    let motion_detector_thread = thread::spawn(move || -> () {
        let mut md = MotionDetector::new(frame_rx);
        md.start();
    });

    frame_reader_thread.join().unwrap();
    motion_detector_thread.join().unwrap();
}
