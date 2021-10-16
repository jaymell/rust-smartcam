extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use log::{debug, warn};
use opencv::core::Mat_AUTO_STEP;
use opencv::core::CV_8UC3;
use opencv::prelude::*;
use std::error::Error;
use std::sync::{mpsc::Sender, Arc};
use std::time::SystemTime;

use crate::frame::{Colorspace, Frame};

pub fn start(
    senders: Vec<Sender<Arc<Frame>>>,
    source: Option<&str>,
    win_tx: Option<glib::Sender<Arc<Frame>>>,
) -> Result<(), Box<dyn Error>> {
    let mut ictx = input(&source.unwrap()).unwrap();
    let input = ictx.streams().best(Type::Video).unwrap();
    let video_stream_index = input.index();

    let mut decoder = input.codec().decoder().video().unwrap();

    debug!(
        "Original dimensions are {} x {}",
        decoder.width(),
        decoder.height()
    );

    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::BGR24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )
    .unwrap();

    let mut receive_and_process_decoded_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();

                scaler.run(&decoded, &mut rgb_frame).unwrap();

                let rgb_frame = rgb_frame.data(0);

                unsafe {
                    let img = Mat::new_rows_cols_with_data(
                        decoder.height() as _,
                        decoder.width() as _,
                        CV_8UC3,
                        // Note: this data is not copied:
                        rgb_frame.as_ptr() as *mut std::os::raw::c_void,
                        Mat_AUTO_STEP,
                    )
                    .unwrap();

                    let frame =
                        Frame::new(img.clone(), Colorspace::BGR, Some(SystemTime::now().into()));
                    let a = Arc::new(frame);
                    for s in &senders {
                        s.send(Arc::clone(&a)).unwrap();
                    }
                    if let Some(tx) = &win_tx {
                        tx.send(Arc::clone(&a)).unwrap();
                    }
                }
            }
            Ok(())
        };

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            match decoder.send_packet(&packet) {
                Ok(_) => (),
                Err(e) => {
                    warn!("Error sending packet: {} -- dropping", e);
                    continue;
                }
            }
            receive_and_process_decoded_frames(&mut decoder).unwrap();
        }
    }

    decoder.send_eof().unwrap();
    receive_and_process_decoded_frames(&mut decoder).unwrap();

    Ok(())
}
