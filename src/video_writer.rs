use ffmpeg_next::{
    codec, codec::encoder::video::Video, codec::id::Id, format, format::context::output::Output,
    format::stream::StreamMut, format::Pixel, frame, util::rational::Rational, Dictionary, Picture,
};
use ffmpeg_sys_next as ffs;
use ffs::{avpicture_fill, AVCodecID::AV_CODEC_ID_RAWVIDEO, AVFrame, AVPixelFormat};
use libc::c_int;
use log::{debug, error, trace};
use opencv::core::prelude::MatTrait;
use std::convert::TryFrom;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::frame::{Frame, VideoFrame};

pub struct VideoWriter {
    sender: Sender<VideoFrame>,
}

impl VideoWriter {
    pub fn new() -> Self {
        let (video_tx, video_rx) = mpsc::channel::<VideoFrame>();

        let thread = thread::spawn(|| -> () {
            let mut video_frame_proc = VideoFrameProcessor::new(video_rx);
            video_frame_proc.receive();
        });

        Self { sender: video_tx }
    }

    pub fn send_frame(&self, frame: VideoFrame) -> () {
        self.sender.send(frame).unwrap();
    }
}

struct VideoFrameProcessor {
    receiver: Receiver<VideoFrame>,
}

impl VideoFrameProcessor {
    pub fn new(receiver: Receiver<VideoFrame>) -> Self {
        Self { receiver: receiver }
    }

    pub fn receive(&mut self) -> () {
        let p = Path::new("/tmp/out.mp4");
        let mut octx = format::output(&p).unwrap();
        let mut ost = octx
            .add_stream(codec::encoder::find(Id::from(AV_CODEC_ID_RAWVIDEO)))
            .unwrap();
        let mut encoder = ost.codec().encoder().video().unwrap();

        // Get first frame:
        let video_frame = self.receiver.recv().unwrap();
        let frame = video_frame.frame;

        encoder.set_width(frame.width);
        encoder.set_height(frame.height);
        encoder.set_format(Pixel::YUYV422);
        // FIXME
        encoder.set_time_base(Rational::new(1, 20));
        let mut x264opts = Dictionary::new();
        encoder.open_with(x264opts).expect("couldn't open encoder");
        // Not sure why encoder is reassigned here:
        encoder = ost.codec().encoder().video().unwrap();

        self.process_frame(frame, &mut encoder);

        loop {
            let video_frame = self.receiver.recv().unwrap();
            let frame = video_frame.frame;
            self.process_frame(frame, &mut encoder);
            if video_frame.is_end {
                encoder.send_eof().unwrap();
                break;
            }
        }
    }

    fn process_frame(&mut self, frame: Frame, encoder: &mut Video) -> () {
        // Picture to AvFrame to ffmpeg_next::Frame to ffmpeg_next::frame::Video ... WTF
        let mut dst = Picture::new(Pixel::YUYV422, frame.width, frame.height).unwrap();
        debug!("width height {:?} {:?}", frame.width as c_int, frame.height as c_int);
        unsafe {
            avpicture_fill(
                dst.as_mut_ptr(),
                frame.img.datastart(),
                AVPixelFormat::from(Pixel::YUYV422),
                frame.width as c_int,
                frame.height as c_int,
            );
        }

        // let avframe = AVFrame::try_from(dst).unwrap();
        let mut ff_frame: frame::Frame;
        unsafe {
            ff_frame = frame::Frame::wrap(dst.as_mut_ptr() as *mut AVFrame);
        }

        let mut video_frame = frame::Video::from(ff_frame);
        video_frame.set_width(frame.width);
        video_frame.set_height(frame.height);
        video_frame.set_format(Pixel::YUYV422);
        encoder.send_frame(&video_frame).unwrap();
    }
}
