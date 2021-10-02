use crate::frame::{Frame, VideoFrame};
use crate::uploader;
use chrono;
use chrono::{DateTime, Utc};
use ffmpeg::{
    codec, codec::encoder::video::Video, format, format::Pixel, frame, util::log::level::Level,
    util::rational::Rational, Dictionary, Packet,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next as ffs;
use ffs::{av_frame_alloc, av_frame_get_buffer, avpicture_fill, AVPicture, AVPixelFormat};
use libc::c_int;
use log::{debug, error, warn};
use opencv::core::prelude::MatTrait;
use std::error::Error;
use std::fs;
use std::mem;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use tokio::runtime::Runtime;

pub struct VideoWriter {
    sender: Sender<VideoFrame>,
}

impl VideoWriter {
    pub fn new() -> Self {
        let (video_tx, video_rx) = mpsc::channel::<VideoFrame>();

        thread::spawn(|| -> () {
            let mut video_frame_proc = VideoFrameProcessor::new(video_rx, 1000);
            match video_frame_proc.receive() {
                Ok(p) => VideoWriter::handle_upload(p),
                Err(e) => error!("Video writing failed: {}", e),
            }
        });

        Self { sender: video_tx }
    }

    pub fn send_frame(&self, frame: VideoFrame) -> () {
        self.sender.send(frame).unwrap();
    }

    fn handle_upload(path: String) -> () {
        match Runtime::new()
            .unwrap()
            .block_on(uploader::upload_file(&path))
        {
            Ok(_) => {
                debug!("Deleting file {}", &path);
                fs::remove_file(path).unwrap();
            }
            Err(e) => {
                error!("File download failed: {}", e);
                warn!(
                    "Skipping deletion due to upload failure; video retained at {}",
                    &path
                );
            }
        }
    }
}

fn parse_opts<'a>(s: String) -> Dictionary<'a> {
    let mut dict = Dictionary::new();
    for keyval in s.split_terminator(',') {
        let tokens: Vec<&str> = keyval.split('=').collect();
        match tokens[..] {
            [key, val] => dict.set(key, val),
            _ => return Dictionary::new(),
        }
    }
    dict
}

struct VideoFrameProcessor {
    receiver: Receiver<VideoFrame>,
    fps: i16,
    start_time: Option<DateTime<Utc>>,
    previous_frame_time: Option<DateTime<Utc>>,
    previous_pts: Option<i64>,
    frame_count: i64,
}

impl VideoFrameProcessor {
    pub fn new(receiver: Receiver<VideoFrame>, fps: i16) -> Self {
        Self {
            receiver: receiver,
            fps: fps,
            start_time: None,
            previous_frame_time: None,
            previous_pts: None,
            frame_count: 0,
        }
    }

    fn image_format() -> Pixel {
        Pixel::BGR24
        // Pixel::YUYV422
        // Pixel::YUV420P
    }

    fn image_format_raw() -> AVPixelFormat {
        AVPixelFormat::AV_PIX_FMT_BGR24
    }

    fn video_format() -> Pixel {
        Pixel::YUV420P
        // Pixel::BGR24
    }

    fn video_format_raw() -> AVPixelFormat {
        AVPixelFormat::AV_PIX_FMT_YUV420P
        // AVPixelFormat::AV_PIX_FMT_BGR24
    }

    pub fn receive(&mut self) -> Result<String, Box<dyn Error>> {
        // Get first frame:
        let video_frame = self.receiver.recv().unwrap();
        let frame = video_frame.frame;
        self.start_time = Some(frame.time());

        let f = format!("/tmp/{}.mkv", frame.time().format("%+"));
        let p = Path::new(&f);

        // ffmpeg::util::log::set_level(Level::Trace);
        ffmpeg::init().unwrap();

        // let x264_opts = parse_opts("enable-debug=3".to_string());
        let x264_opts = parse_opts("".to_string());

        let mut octx = format::output(&p).unwrap();

        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);

        let ost = octx
            .add_stream(codec::encoder::find(codec::Id::H264))
            .unwrap();
        let mut encoder = ost.codec().encoder().video().unwrap();

        encoder.set_width(frame.width());
        encoder.set_height(frame.height());
        encoder.set_format(VideoFrameProcessor::video_format());
        encoder.set_time_base(Rational::new(1, self.fps.into()));

        if global_header {
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        encoder.open_with(x264_opts).expect("couldn't open encoder");
        // Not sure why encoder is reassigned here:
        encoder = ost.codec().encoder().video().unwrap();
        // Not sure what this is doing
        format::context::output::dump(&octx, 0, Some(&f));
        octx.write_header().unwrap();

        self.process_frame(frame, &mut encoder);
        self.receive_and_process_encoded_packets(&mut octx, &mut encoder);

        loop {
            let video_frame = self.receiver.recv().unwrap();
            let frame = video_frame.frame;
            self.process_frame(frame, &mut encoder);
            self.receive_and_process_encoded_packets(&mut octx, &mut encoder);
            if video_frame.is_end {
                debug!("Last frame receieved, sending EOF");
                encoder.send_eof().unwrap();
                octx.write_trailer().unwrap();
                break;
            }
        }

        Ok(p.to_str().unwrap().to_string())
    }

    fn process_frame(&mut self, mut frame: Frame, encoder: &mut Video) -> () {
        unsafe {
            let mut dst = av_frame_alloc();
            (*dst).width = frame.width() as _;
            (*dst).height = frame.height() as _;
            (*dst).format = mem::transmute::<AVPixelFormat, c_int>(
                VideoFrameProcessor::image_format_raw().into(),
            );
            av_frame_get_buffer(dst, 32);
            avpicture_fill(
                dst as *mut AVPicture,
                frame.img_mut().data_mut(),
                AVPixelFormat::from(VideoFrameProcessor::image_format()),
                encoder.width() as _,
                encoder.height() as _,
            );
            debug!("Buffer size is {:?}", (*(*dst).buf[0]).size);
            let mut video_frame = frame::Video::wrap(dst);
            video_frame.set_width(frame.width());
            video_frame.set_height(frame.height());
            video_frame.set_format(VideoFrameProcessor::image_format());

            let mut converted = frame::Video::empty();
            converted.set_width(frame.width());
            converted.set_height(frame.height());
            converted.set_format(VideoFrameProcessor::video_format());

            video_frame
                .converter(VideoFrameProcessor::video_format())
                .unwrap()
                .run(&video_frame, &mut converted)
                .unwrap();
            let pts = Some(self.calc_pts(frame.time()).unwrap_or(0));
            converted.set_pts(pts);

            encoder.send_frame(&converted).unwrap();

            self.previous_frame_time = Some(frame.time());
            self.previous_pts = pts;
            self.frame_count = self.frame_count + 1;
        }
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
        encoder: &mut Video,
    ) {
        let ost_index = 0;
        let mut encoded = Packet::empty();
        while encoder.receive_packet(&mut encoded).is_ok() {
            debug!("Writing packets...");
            encoded.set_stream(ost_index);
            encoded.write_interleaved(octx).unwrap();
        }
        debug!("Finished writing packets...");
    }

    fn calc_pts(&mut self, ts: DateTime<Utc>) -> Option<i64> {
        match self.previous_frame_time {
            Some(t) => {
                match self.previous_pts {
                    Some(p) => {
                        let delta = (ts - t).num_milliseconds();
                        // let result = ((delta as f64) / self.fps as f64).round() as i64 + p;
                        // let result = (self.frame_count as f64 / self.fps as f64).round() as i64;
                        // let result = p + 300;
                        let result =
                            ((delta as f64 / 1000.0) * self.fps as f64 + p as f64).round() as i64;
                        debug!("PTS {:?}", result);
                        Some(result)
                    }
                    None => {
                        warn!("Last PTS not found.");
                        None
                    }
                }
            }
            None => None,
        }
    }
}
