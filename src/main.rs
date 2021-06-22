use opencv::{
	highgui,
	prelude::*,
	Result,
	videoio,
	core::Point,
	core::Scalar_,
  core::Size_,
  core::absdiff,
  core::BORDER_CONSTANT,
  core::BORDER_DEFAULT,
  imgproc::threshold,
  imgproc::dilate,
  imgproc::morphology_default_border_value,
  imgproc::find_contours,
  imgproc::gaussian_blur,
  imgproc::RETR_TREE,
  imgproc::CHAIN_APPROX_SIMPLE,
  imgproc::THRESH_BINARY,
  imgproc::COLOR_BGR2GRAY,
  imgproc::cvt_color,
  types::VectorOfMat
};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime};
use chrono::{DateTime, Utc};


struct Frame {
	img: Mat,
	pub time: DateTime<Utc>,
	pub height: i32,
	pub width: i32

}

impl Frame {

    fn get_img(&self) -> &Mat {
      &self.img
    }

    fn blur(&self) -> Result<Frame> {
      let mut blurred = Mat::default();
      gaussian_blur(&self.img, &mut blurred, Size_::new(21, 21), 0.0, 0.0, BORDER_DEFAULT)?;
      Ok(Frame { img: blurred, ..*self })
    }

    fn grayscale(&self) -> Result<Frame> {
      let mut gray = Mat::default();
      cvt_color(&self.img, &mut gray, COLOR_BGR2GRAY, 0)?;
      Ok(Frame { img: gray, ..*self })
    }

    fn downsample(&self) -> Result<Frame> {
      self
        .grayscale()?
        .blur()
    }

}

impl Clone for Frame {

    fn clone(&self) -> Frame {
      Frame {
        img: self.img.clone(),
        time: self.time,
        height: self.height,
        width: self.width
      }
    }

}


fn main() -> Result<()> {


#[cfg(ocvrs_opencv_branch_32)]
let mut cam = videoio::VideoCapture::new_default(0)?; // 0 is the default camera

#[cfg(not(ocvrs_opencv_branch_32))]
let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 is the default camera

let opened = videoio::VideoCapture::is_opened(&cam)?;
if !opened {
	panic!("Unable to open default camera!");
}


let (frame_tx, frame_rx) = mpsc::channel::<Frame>();
let (viewer_tx, viewer_rx) = mpsc::channel::<Frame>();
let (motion_tx, motion_rx) = mpsc::channel::<Frame>();

let frame_viewer = thread::spawn(move || -> Result<()> {

		let fps = 1000 / 50;

		// let window = "video capture";
  //   println!("opening video capture window");
		// highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
  //   println!("opening video capture window DONE");

	  let font = highgui::font_qt("", 12, Scalar_::new(40.0, 252.0, 3.0, 0.0),
	  	highgui::QT_FONT_NORMAL, highgui::QT_STYLE_NORMAL, 0)?;

  	loop {
  		let frame = viewer_rx.recv().unwrap();
			// highgui::add_text(&frame.get_img(), &frame.time.to_rfc3339(), Point::new(frame.width/3,frame.height-(frame.height/6)), &font)?;
	  //   highgui::imshow(window, &frame.get_img())?;
	  //   highgui::wait_key(1)?;
  	}

  	Ok(())

  });


  let frame_reader = thread::spawn(move || -> Result<()> {

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

			frame_tx.send(frame).unwrap();

    }

    Ok(())

  });


  let frame_splitter = thread::spawn(move || -> Result<()> {

    loop {
      let frame = frame_rx.recv().unwrap();
      viewer_tx.send(frame.clone()).unwrap();
      motion_tx.send(frame).unwrap();
    }

    Ok(())

  });


  let motion_detector = thread::spawn(move || -> Result<()> {

    let mut previous = motion_rx.recv().unwrap().downsample()?;

    let window = "motion detection";
    println!("opening motion detection window");
    highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
    println!("opening motion detection window DONE");

    loop {

      let frame = motion_rx.recv().unwrap().downsample()?;

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

  });


  frame_reader.join().unwrap();
  frame_viewer.join().unwrap();
  frame_splitter.join().unwrap();
  motion_detector.join().unwrap();

	Ok(())
}
