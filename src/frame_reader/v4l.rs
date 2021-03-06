use super::FrameReader;
use log::{debug, info};
use opencv::core::Mat_AUTO_STEP;
use opencv::core::CV_8UC3;
use opencv::prelude::*;
use std::sync::{mpsc::Sender, Arc};
use std::time::SystemTime;
use tokio::sync::mpsc::Sender as AsyncSender;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;

use crate::frame::{Colorspace, Frame};

pub struct V4LFrameReader {}

impl FrameReader for V4LFrameReader {
    fn read_frames(
        &self,
        senders: Vec<Sender<Arc<Frame>>>,
        web_tx: Option<AsyncSender<Arc<Frame>>>,
        source: Option<&str>,
    ) {
        if senders.len() == 0 {
            panic!("No frame recipients specified");
        }

        // FIXME -- get this smarter:
        let path = match source {
            Some(p) => p,
            None => "/dev/video0",
        };

        info!("v4l reader using device: {}\n", path);

        // Allocate 4 buffers by default
        let buffer_count = 4;
        let mut dev = Device::with_path(path).unwrap();
        let format = Device::with_path(path).unwrap().format().unwrap();
        let fourcc = format.fourcc;
        debug!("fourcc: {}", fourcc);
        debug!("width: {}", format.width);
        debug!("height: {}", format.height);
        let mut stream =
            MmapStream::with_buffers(&mut dev, Type::VideoCapture, buffer_count).unwrap();

        loop {
            let (buf, _meta) = stream.next().unwrap();
            if buf.len() == 0 {
                continue;
            }

            let mut bgr_buf = Colorspace::str(&fourcc.str().unwrap())
                .unwrap()
                .convert_buf(buf.to_vec(), Colorspace::BGR);

            let img = unsafe {
                Mat::new_rows_cols_with_data(
                    format.height as _,
                    format.width as _,
                    CV_8UC3,
                    bgr_buf.as_mut_ptr() as *mut std::os::raw::c_void,
                    Mat_AUTO_STEP,
                )
                .unwrap()
            };
            let frame = Frame::new(img.clone(), Colorspace::BGR, Some(SystemTime::now().into()));
            if frame.width() == 0 {
                continue;
            }

            let a = Arc::new(frame);
            for s in &senders {
                s.send(Arc::clone(&a)).unwrap();
            }
            if let Some(s) = &web_tx {
                s.blocking_send(Arc::clone(&a)).unwrap();
            }
        }
    }
}
