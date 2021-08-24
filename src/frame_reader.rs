use std::sync::mpsc::Sender;

use opencv::{prelude::*, videoio, Result};

use chrono::{DateTime, Utc};
use std::time::SystemTime;

use crate::frame::Frame;

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

        let frame = Frame::new(img, None);
        if frame.width() == 0 {
            continue;
        }

        sender.send(frame).unwrap();
    }

    Ok(())
}
