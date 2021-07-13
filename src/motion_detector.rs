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
  imgproc::contour_area,
  types::VectorOfMat
};
use chrono::{DateTime, NaiveDateTime, Utc, Duration};
use log::{debug, trace};

use crate::core::Frame;

const MIN_CONTOUR_AREA: i16 = 200;


pub fn start(receiver: Receiver<Frame>) -> Result<()> {

  let mut previous = receiver.recv().unwrap().downsample()?;

  let window = "motion detection";
  debug!("opening motion detection window");
  highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
  debug!("opening motion detection window DONE");

  let mut in_motion = false;
  let mut in_motion_window = false;
  let mut last_motion_time = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc);

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

    for c in contours.iter() {
      trace!("Contours: {:?}", c);
      let area = contour_area(&c, false).unwrap();
      if area as i16 >= MIN_CONTOUR_AREA {
        in_motion = true;
        in_motion_window = true;
        last_motion_time = frame.time;
        debug!("Motion detected at {:?}", last_motion_time);
        break;
      }
    }

    if in_motion_window && !check_in_motion(frame.time, last_motion_time) {
      debug!("Motion window closing.");
      in_motion_window = false;
    }

    highgui::imshow(window, &dilated)?;
    highgui::wait_key(1)?;

    previous = frame;
  }

  Ok(())
}


fn check_in_motion(current_time: DateTime<Utc>, last_motion_time: DateTime<Utc>) -> bool {
  let min_motion_capture_time: Duration = Duration::seconds(20);
  if ( current_time - min_motion_capture_time) >= last_motion_time {
    false
  } else {
    true
  }
}
