use std::sync::mpsc::Receiver;
use opencv::{
  highgui,
  prelude::*,
  Result,
  core::Point,
  core::absdiff,
  core::BORDER_CONSTANT,
  imgproc::threshold,
  imgproc::dilate,
  imgproc::morphology_default_border_value,
  imgproc::find_contours,
  imgproc::RETR_TREE,
  imgproc::CHAIN_APPROX_SIMPLE,
  imgproc::THRESH_BINARY,
  types::VectorOfMat
};

use crate::core::Frame;

pub fn start(receiver: Receiver<Frame>) -> Result<()> {
  let mut previous = receiver.recv().unwrap().downsample()?;

  let window = "motion detection";
  println!("opening motion detection window");
  highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
  println!("opening motion detection window DONE");

  loop {

    let frame = receiver.recv().unwrap().downsample()?;

    let mut delta = Mat::default();
    absdiff(&previous.get_img(), &frame.get_img(), &mut delta);

    let mut thresh = Mat::default();
    threshold(&delta, &mut thresh, 25.0, 255.0, THRESH_BINARY);

    let mut dilated = Mat::default();
    dilate(&thresh, &mut dilated, &Mat::default(), Point::new(1,1), 1, BORDER_CONSTANT, morphology_default_border_value().unwrap());

    let mut contours = VectorOfMat::new();

    find_contours(&dilated, &mut contours, RETR_TREE, CHAIN_APPROX_SIMPLE, Point::new(0,0))?;

    // println!("Contours: {:?}", contours);

    highgui::imshow(window, &dilated)?;
    highgui::wait_key(1)?;

    previous = frame;
  }

  Ok(())
}


