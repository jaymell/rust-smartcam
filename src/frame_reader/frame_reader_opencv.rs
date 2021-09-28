use opencv::{prelude::*, videoio, Result};
use std::sync::mpsc::Sender;
use std::time::Instant;
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

    let mut frame_count = 0;
    let start = Instant::now();
    loop {
        frame_count += 1;
        let mut img = Mat::default();
        cam.read(&mut img)?;

        let frame = Frame::new(img, Some(SystemTime::now().into()));
        if frame.width() == 0 {
            continue;
        }

        sender.send(frame).unwrap();

        println!(
            "FPS: {}",
            frame_count as f64 / start.elapsed().as_secs_f64()
        );
    }

    Ok(())
}
