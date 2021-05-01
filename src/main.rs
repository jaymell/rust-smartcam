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

fn main() -> Result<()> {


	#[cfg(ocvrs_opencv_branch_32)]
	let mut cam = videoio::VideoCapture::new_default(0)?; // 0 is the default camera

	#[cfg(not(ocvrs_opencv_branch_32))]
	let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 is the default camera

	let opened = videoio::VideoCapture::is_opened(&cam)?;
	if !opened {
		panic!("Unable to open default camera!");
	}




  let (tx, rx) = mpsc::channel::<Mat>();

  thread::spawn(move || -> Result<()> {

		let fps = 1000 / 5;

		let window = "video capture";
		highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;

	  let font = highgui::font_qt("", 12, Scalar_::new(40.0, 252.0, 3.0, 0.0),
	  	highgui::QT_FONT_NORMAL, highgui::QT_STYLE_NORMAL, 0)?;

    loop {
	    println!("Getting frame");
	    let frame = rx.recv().unwrap();
	    println!("Got frame");
			let now = SystemTime::now();
			let now: DateTime<Utc> = now.into();
	    let now = now.to_rfc3339();

			let width = frame.size()?.width;
			let height = frame.size()?.height;

			highgui::add_text(&frame, &now, Point_::new(width/3,height-(height/6)), &font)?;
	    highgui::imshow(window, &frame)?;
	    highgui::wait_key(fps)?;
    }

    Ok(())

  });




	loop {

		let mut frame = Mat::default();
		println!("Reading frame. ");
		cam.read(&mut frame)?;
		println!("Read frame. ");
		let width = frame.size()?.width;
		let height = frame.size()?.height;
		if width > 0 {
			println!("Sending frame. ");
			tx.send(frame.clone()).unwrap();
			println!("Sent frame. ");
		} else {
			println!("Width is all wrong man")
		}
	}

	Ok(())
}
