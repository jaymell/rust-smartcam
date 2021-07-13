use std::sync::mpsc::Sender;

use opencv::{
  prelude::*,
  Result,
  videoio,
};

use std::time::{SystemTime};
use chrono::{DateTime, Utc};

use crate::core::Frame;


pub fn start(sender: Sender<Frame>) -> Result<()> {

  #[cfg(ocvrs_opencv_branch_32)]
  let mut cam = videoio::VideoCapture::new_default(0)?; // 0 is the default camera

  #[cfg(not(ocvrs_opencv_branch_32))]
  let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 is the default camera

  let opened = videoio::VideoCapture::is_opened(&cam)?;
  if !opened {
    panic!("Unable to open default camera!");
  }

  loop {

    let mut img = Mat::default();
    cam.read(&mut img)?;

    let now: DateTime<Utc> = SystemTime::now().into();

    let frame = Frame {
      time: now,
      width: img.size()?.width,
      height: img.size()?.height,
      img: img
    };

    if frame.width == 0 {
      continue;
    }

    sender.send(frame).unwrap();

  }

 Ok(())


}

