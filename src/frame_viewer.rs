use opencv::{core::Point_, core::Scalar_, highgui};
use std::sync::mpsc::Receiver;

use crate::frame::Frame;

pub fn start(receiver: Receiver<Frame>) -> () {
    let window = "video capture";
    println!("opening video capture window");
    highgui::named_window(window, highgui::WINDOW_AUTOSIZE).unwrap();
    println!("opening video capture window DONE");

    let font = highgui::font_qt(
        "",
        12,
        Scalar_::new(40.0, 252.0, 3.0, 0.0),
        highgui::QT_FONT_NORMAL,
        highgui::QT_STYLE_NORMAL,
        0,
    )
    .unwrap();

    loop {
        let frame = receiver.recv().unwrap();
        highgui::add_text(
            &frame.img(),
            &frame.time().to_rfc3339(),
            Point_::new(
                frame.width() as i32 / 3,
                (frame.height() - (frame.height() / 6)) as i32,
            ),
            &font,
        )
        .unwrap();
        highgui::imshow(window, &frame.img()).unwrap();
        highgui::wait_key(1).unwrap();
    }
}
