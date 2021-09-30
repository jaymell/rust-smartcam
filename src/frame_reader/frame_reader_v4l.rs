use opencv::core::Mat_AUTO_STEP;
use opencv::core::CV_8UC3;
use opencv::{prelude::*};
use std::convert::TryInto;
use std::error::Error;
use std::sync::mpsc::Sender;
use std::time::Instant;
use std::time::SystemTime;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;

use crate::frame::Frame;


pub enum Format {
    YUYV,
}

fn yuyv_to_bgr(buf: &[u8]) -> Result<Vec<u8>, Box<Error>> {
    /*
    Cr aka V aka red
    Cb aka U aka blue
    R = Y + 1.402 (Cr-128.0)
    G = Y - 0.34414 (Cb-128.0) - 0.71414 (Cr-128.0)
    B = Y + 1.772 (Cb-128.0)
    */

    let mut mat_buf = Vec::new();

    buf.windows(4).for_each(|s| {
        let [y1, u, y2, v]: [u8; 4] = s.try_into().unwrap();
        let y1 = y1 as f64;
        let u = u as f64;
        let y2 = y2 as f64;
        let v = v as f64;

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
    });
    Ok(mat_buf)
}

fn to_bgr(buf: &[u8], fourcc: &str) -> Result<Vec<u8>, Box<Error>> {
    match fourcc {
        "YUYV" => yuyv_to_bgr(buf),
        _ => panic!("Not supported"),
    }
}

pub fn start(senders: Vec<Sender<Frame>>) -> Result<(), Box<Error>> {
    if senders.len() == 0 {
        panic!("No frame recipients specified");
    }

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

            if senders.len() == 1 {
                senders[0].send(frame).unwrap();
            } else {
                for s in &senders {
                    let f = frame.clone();
                    s.send(f).unwrap();
                }
            }
        }

        frame_count += 1;
        println!(
            "FPS: {}",
            frame_count as f64 / start.elapsed().as_secs_f64()
        );
    }

    Ok(())
}
