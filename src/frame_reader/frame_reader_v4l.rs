use log::{debug, warn};
use opencv::core::Mat_AUTO_STEP;
use opencv::core::Scalar;
use opencv::core::{CV_8UC1, CV_8UC2, CV_8UC3};
use opencv::{prelude::*, videoio, Result};
use std::sync::mpsc::Sender;
use std::time::Instant;
use std::time::SystemTime;

use crate::frame::Frame;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::FourCC;

pub enum Format {
    YUYV,
}

fn yuyv_to_bgr(buf: &[u8]) -> Result<Vec<u8>> {
    /*
    Cr aka V aka red
    Cb aka U aka blue
    R = Y + 1.402 (Cr-128.0)
    G = Y - 0.34414 (Cb-128.0) - 0.71414 (Cr-128.0)
    B = Y + 1.772 (Cb-128.0)
    */

    let dst_len = buf.len() as f64 * 1.5;
    let mut mat_buf = Vec::new();

    let mut i = 0;
    while i < buf.len() {
        let y1 = buf[i] as f64;
        let u = buf[i + 1] as f64;
        let y2 = buf[i + 2] as f64;
        let v = buf[i + 3] as f64;

        let p1_b = (y1 + (1.772 * (u - 128.0))) as u8;
        let p1_g = (y1 - (0.34414 * (u - 128.0)) - (0.71414 * (v - 128.0))) as u8;
        let p1_r = (y1 + 1.402 * (v - 128.0)) as u8;

        let p2_b = (y2 + (1.772 * (u - 128.0))) as u8;
        let p2_g = (y2 - (0.34414 * (u - 128.0)) - (0.71414 * (v - 128.0))) as u8;
        let p2_r = (y2 + 1.402 * (v - 128.0)) as u8;

        mat_buf.push(p1_b);
        mat_buf.push(p1_g);
        mat_buf.push(p1_r);
        mat_buf.push(p2_b);
        mat_buf.push(p2_g);
        mat_buf.push(p2_r);

        i = i + 4;
    }
    Ok(mat_buf)
}

fn to_bgr(buf: &[u8], fourcc: &str) -> Result<Vec<u8>> {
    match fourcc {
        "YUYV" => yuyv_to_bgr(buf),
        _ => panic!("Not supported"),
    }
}

pub fn start(sender: Sender<Frame>) -> Result<()> {
    // FIXME -- get this smarter:
    let path = "/dev/video0";
    println!("Using device: {}\n", path);

    // Allocate 4 buffers by default
    let buffer_count = 4;
    let mut dev = Device::with_path(path).unwrap();
    let format = Device::with_path(path).unwrap().format().unwrap();
    let fourcc = format.fourcc;
    println!("fourcc: {}", fourcc);

    let mut stream = MmapStream::with_buffers(&mut dev, Type::VideoCapture, buffer_count).unwrap();

    // Dump first images just b/c:
    for _ in 1..10 {
        stream.next().unwrap();
    }

    let mut frame_count = 0;
    let start = Instant::now();

    loop {
        let (mut buf, meta) = stream.next().unwrap();
        if buf.len() == 0 {
            continue;
        }

        let mut bgr_buf = to_bgr(buf, fourcc.str().unwrap()).unwrap();

        unsafe {
            let mut img = Mat::new_rows_cols_with_data(
                format.height as _,
                format.width as _,
                CV_8UC3,
                bgr_buf.as_mut_ptr() as *mut std::os::raw::c_void,
                Mat_AUTO_STEP,
            )
            .unwrap();
            let frame = Frame::new(img, Some(SystemTime::now().into()));
            if frame.width() == 0 {
                continue;
            }

            sender.send(frame).unwrap();
        }

        frame_count += 1;
        println!(
            "FPS: {}",
            frame_count as f64 / start.elapsed().as_secs_f64()
        );
    }

    Ok(())
}
