use opencv::{
	highgui,
	prelude::*,
	Result,
	videoio,
	core::Point_,
	core::Scalar_
};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime};
use chrono::{DateTime, Utc};

struct Frame {
	pub frame: Mat,
	pub time: DateTime<Utc>,
	pub height: i32,
	pub width: i32
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


  let frame_viewer = thread::spawn(move || -> Result<()> {

		let fps = 1000 / 50;

		let window = "video capture";
		highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;

	  let font = highgui::font_qt("", 12, Scalar_::new(40.0, 252.0, 3.0, 0.0),
	  	highgui::QT_FONT_NORMAL, highgui::QT_STYLE_NORMAL, 0)?;

  	loop {
  		let frame = frame_rx.recv().unwrap();
			highgui::add_text(&frame.frame, &frame.time.to_rfc3339(), Point_::new(frame.width/3,frame.height-(frame.height/6)), &font)?;
	    highgui::imshow(window, &frame.frame)?;
	    highgui::wait_key(1)?;
  	}

  	Ok(())

  });


  let frame_reader = thread::spawn(move || -> Result<()> {

    loop {

			let mut frame = Mat::default();
			cam.read(&mut frame)?;

			let now: DateTime<Utc> = SystemTime::now().into();

	    let frame = Frame {
	    	time: now,
	    	width: frame.size()?.width,
	    	height: frame.size()?.height,
	    	frame: frame
	    };

			if frame.width == 0 {
				continue;
			}

			frame_tx.send(frame).unwrap();

    }

    Ok(())

  });



  frame_reader.join().unwrap();
  frame_viewer.join().unwrap();

	Ok(())
}
