extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use log::{debug, warn};
use opencv::core::Mat_AUTO_STEP;
use opencv::core::{CV_8UC1, CV_8UC2, CV_8UC3};
use opencv::prelude::*;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::sync::mpsc::Sender;
use std::time::Instant;
use std::time::SystemTime;

use ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_RGB24;
use ffmpeg_sys_next::{av_image_copy_to_buffer, av_image_get_buffer_size};

use crate::frame::Frame;

/// swap red and blue -- not needed but used for timing comparison:
fn rgb_to_bgr(buf: &mut [u8]) -> Result<(), Box<Error>> {
    let mut i = 0;

    while i < buf.len() {
        let temp = buf[i];
        buf[i] = buf[i + 2];
        buf[i + 2] = temp;

        i = i + 3;
    }
    Ok(())
}

pub fn start(senders: Vec<Sender<Frame>>, source: &String) -> Result<(), Box<Error>> {
    let mut ictx = input(&source).unwrap();
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
        // Pixel::RGB24,
        Pixel::BGR24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )
    .unwrap();

    let mut i = 0;
    let mut receive_and_process_decoded_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();

                scaler.run(&decoded, &mut rgb_frame).unwrap();

                let mut rgb_frame = rgb_frame.data_mut(0);

                unsafe {
                    let mut img = Mat::new_rows_cols_with_data(
                        decoder.height() as _,
                        decoder.width() as _,
                        CV_8UC3,
                        rgb_frame.as_mut_ptr() as *mut std::os::raw::c_void,
                        Mat_AUTO_STEP,
                    )
                    .unwrap();

                    let frame = Frame::new(img, Some(SystemTime::now().into()));

                    if senders.len() == 1 {
                        senders[0].send(frame).unwrap();
                    } else {
                        for s in &senders {
                            let f = frame.clone();
                            s.send(f).unwrap();
                        }
                    }
                }
            }
            Ok(())
        };

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet).unwrap();
            receive_and_process_decoded_frames(&mut decoder).unwrap();
        }
    }

    decoder.send_eof().unwrap();
    receive_and_process_decoded_frames(&mut decoder).unwrap();

    Ok(())
}
