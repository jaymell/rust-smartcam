// pub struct FrameViewer {
// }

// let frame_viewer = thread::spawn(move || -> Result<()> {


//     // let window = "video capture";
//   //   println!("opening video capture window");
//     // highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
//   //   println!("opening video capture window DONE");

//     let font = highgui::font_qt("", 12, Scalar_::new(40.0, 252.0, 3.0, 0.0),
//       highgui::QT_FONT_NORMAL, highgui::QT_STYLE_NORMAL, 0)?;

//     loop {
//       let frame = viewer_rx.recv().unwrap();
//       // highgui::add_text(&frame.get_img(), &frame.time.to_rfc3339(), Point::new(frame.width/3,frame.height-(frame.height/6)), &font)?;
//     //   highgui::imshow(window, &frame.get_img())?;
//     //   highgui::wait_key(1)?;
//     }

//     Ok(())

//   });