use super::init_encoder;
use super::{RTCTrack, VideoProc};
use crate::config;
use crate::config::CameraConfig;
use crate::frame::{Frame, VideoFrame};
use crate::upload;
use crate::FileSourceType;

use bytes::Bytes;
use chrono;
use chrono::{DateTime, Duration, Utc};
use ffmpeg::{
    codec, codec::encoder::video::Video, format, format::context::output::Output,
    format::stream::StreamMut, format::Pixel, frame, util::log::level::Level,
    util::rational::Rational, Dictionary, Packet,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next as ffs;
use ffs::{
    av_frame_alloc, av_frame_get_buffer, av_guess_format, avformat_alloc_context, avpicture_fill,
    AVPicture, AVPixelFormat,
};
use libc::c_int;
use log::{debug, error, info, trace, warn};
use opencv::core::prelude::MatTrait;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::CString;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel as async_channel, Receiver as AsyncReceiver};
use tokio::sync::Mutex;
use webrtc::api::media_engine::MIME_TYPE_H264;
use webrtc::media::Sample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;

pub struct VideoFileWriter {
    start_time: DateTime<Utc>,
    video_proc: VideoProc,
    path: PathBuf,
    temp_path: &'static str,
}

impl VideoFileWriter {
    // FIXME -- return result:
    pub fn new(
        label: String,
        start_time: DateTime<Utc>,
        width: u32,
        height: u32,
        fps: Option<i16>,
    ) -> Self {
        let temp_path = "/tmp";
        let config = config::load_config(None);
        let f_name = format!(
            "{}-{}.{}",
            label,
            start_time.format("%+"),
            config.storage.video_file_type.extension()
        );
        let f = match config.storage.storage_type {
            FileSourceType::Local => format!("{}/{}", config.storage.path, f_name),
            _ => format!("{}/{}", temp_path, f_name),
        };

        let p = Path::new(&f).to_path_buf();
        let fps = fps.unwrap_or(1000);
        let mut octx = format::output(&p).unwrap();
        let mut encoder = init_encoder(width, height, &mut octx, fps, true);

        format::context::output::dump(&octx, 0, Some(&f));
        octx.write_header().unwrap();

        Self {
            start_time: start_time,
            video_proc: VideoProc::new(fps, octx, encoder),
            path: p,
            temp_path: temp_path,
        }
    }

    fn fps(&self) -> i16 {
        self.video_proc.fps()
    }

    fn close_file(&mut self) {
        self.video_proc.encoder.send_eof().unwrap();
        self.video_proc.octx_mut().write_trailer().unwrap();
    }

    fn temp_path(&self) -> &str {
        &self.temp_path
    }

    pub fn receive_file(
        &mut self,
        receiver: Receiver<VideoFrame>,
    ) -> Result<String, Box<dyn Error>> {
        loop {
            let video_frame = receiver.recv().unwrap();
            let frame = video_frame.frame;
            let frame_duration = self.video_proc.process_frame(frame);
            debug!("Frame duration: {:?}", frame_duration);
            self.write_packets_to_ctx();
            if video_frame.is_end {
                debug!("Last frame receieved, sending EOF");
                self.close_file();
                break;
            }
        }

        Ok(self.path.to_str().unwrap().to_string())
    }

    fn write_packets_to_ctx(&mut self) {
        let ost_index = 0;
        let mut encoded = Packet::empty();
        while self.video_proc.encoder.receive_packet(&mut encoded).is_ok() {
            trace!("Writing packets...");
            encoded.set_stream(ost_index);
            encoded
                .write_interleaved(&mut self.video_proc.octx_mut())
                .unwrap();
        }
        trace!("Finished writing packets...");
    }
}
