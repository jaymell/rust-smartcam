use std::{
  sync::mpsc,
  thread
};

mod frame_viewer;
mod frame_reader;
mod frame_splitter;
mod motion_detector;
mod core;
mod logger;

use crate::core::Frame;

fn main() -> () {

  logger::init().unwrap();

  let (frame_tx, frame_rx) = mpsc::channel::<Frame>();

  let frame_reader_thread = thread::spawn(move || -> () {
    frame_reader::start(frame_tx);

  });

  let motion_detector_thread = thread::spawn(move || -> () {
    motion_detector::start(frame_rx);

  });


  frame_reader_thread.join().unwrap();
  motion_detector_thread.join().unwrap();

}
