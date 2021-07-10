use opencv::{
  Result
};

use std::{
  sync::mpsc,
  thread
};


mod frame_viewer;
mod frame_reader;
mod frame_splitter;
mod motion_detector;
mod core;


use crate::core::Frame;

fn main() -> Result<()> {


  let (frame_tx, frame_rx) = mpsc::channel::<Frame>();

  let frame_reader_thread = thread::spawn(move || -> Result<()> {
    frame_reader::start(frame_tx);
    Ok(())

  });

  let motion_detector_thread = thread::spawn(move || -> Result<()> {
    motion_detector::start(frame_rx);
    Ok(())

  });


  frame_reader_thread.join().unwrap();
  motion_detector_thread.join().unwrap();

	Ok(())
}
