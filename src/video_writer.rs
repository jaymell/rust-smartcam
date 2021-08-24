use crate::frame::{Frame, VideoFrame};
use ffmpeg_next::{
    codec, codec::encoder::video::Video, codec::id::Id, format, format::context::output::Output,
    format::stream::StreamMut, format::Pixel, frame, util::rational::Rational, Dictionary, Packet,
    Picture,
};
use ffmpeg_sys_next as ffs;
use ffs::{
    av_frame_alloc, av_frame_get_buffer, avpicture_fill, AVCodecID::AV_CODEC_ID_RAWVIDEO, AVFrame,
    AVPicture, AVPixelFormat, AVPixelFormat::AV_PIX_FMT_BGR24, AVPixelFormat::AV_PIX_FMT_YUYV422,
};
use libc::c_int;
use log::{debug, error, trace};
use opencv::core::prelude::MatTrait;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::mem;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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

    fn format() -> Pixel {
        // Pixel::BGR24
        Pixel::YUYV422
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
        let mut frame = video_frame.frame;

        encoder.set_width(frame.width());
        encoder.set_height(frame.height());
        encoder.set_format(VideoFrameProcessor::format());
        // FIXME
        encoder.set_time_base(Rational::new(1, 20));
        let mut x264opts = Dictionary::new();
        encoder.open_with(x264opts).expect("couldn't open encoder");
        // Not sure why encoder is reassigned here:
        encoder = ost.codec().encoder().video().unwrap();

        self.process_frame(frame, &mut encoder);
        self.receive_and_process_encoded_packets(&mut octx, &mut encoder);

        loop {
            let video_frame = self.receiver.recv().unwrap();
            let mut frame = video_frame.frame;
            let av_frame = self.process_frame(frame, &mut encoder);
            self.receive_and_process_encoded_packets(&mut octx, &mut encoder);
            if video_frame.is_end {
                encoder.send_eof().unwrap();
                break;
            }
        }
    }

    fn process_frame(&mut self, mut frame: Frame, encoder: &mut Video) -> () {
        unsafe {
            let mut dst = av_frame_alloc();
            (*dst).width = frame.width() as c_int;
            (*dst).height = frame.height() as c_int;
            (*dst).format = mem::transmute::<AVPixelFormat, c_int>(AV_PIX_FMT_YUYV422.into());
            // (*dst).format = mem::transmute::<AVPixelFormat, c_int>(AV_PIX_FMT_BGR24.into());
            av_frame_get_buffer(dst, 32);
            avpicture_fill(
                dst as *mut AVPicture,
                frame.img_mut().data_mut(),
                AVPixelFormat::from(VideoFrameProcessor::format()),
                encoder.width() as i32,
                encoder.height() as i32,
            );
            debug!("pre buf");
            let buf = (*dst).buf;
            debug!("post buf");
            debug!("buf size {:?}", (*buf[0]).size);
            debug!("post buf size log");
            let mut video_frame = frame::Video::wrap(dst);
            video_frame.set_width(frame.width());
            video_frame.set_height(frame.height());
            video_frame.set_format(VideoFrameProcessor::format());
            encoder.send_frame(&video_frame).unwrap();
        }
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
        // ost_time_base: Rational,
        encoder: &mut Video,
    ) {
        let ost_index = 0;
        let mut encoded = Packet::empty();
        while encoder.receive_packet(&mut encoded).is_ok() {
            debug!("Setting stream...");
            encoded.set_stream(ost_index);
            // encoded.rescale_ts(self.decoder.time_base(), ost_time_base);
            debug!("Writing packets...");
            encoded.write_interleaved(octx).unwrap();
        }
        debug!("Finished writing packets...");
    }
}
