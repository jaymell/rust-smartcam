mod file_writer;
mod rtc_stream;
pub mod rtc_track;
mod video_proc;

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
use rtc_track::RTCTrack;
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

pub(crate) use file_writer::VideoFileWriter;
pub(crate) use rtc_stream::VideoRTCStream;
pub(crate) use video_proc::VideoProc;

pub fn start_video_writer(
    camera: Arc<CameraConfig>,
    start_time: DateTime<Utc>,
    width: u32,
    height: u32,
) -> Sender<VideoFrame> {
    let (video_tx, video_rx) = mpsc::channel::<VideoFrame>();

    let label = camera.label.clone();
    thread::spawn(move || -> () {
        let app_config = config::load_config(None);
        let mut video_frame_proc = VideoFileWriter::new(label, start_time, width, height, None);
        match video_frame_proc.receive_file(video_rx) {
            Ok(p) => {
                if let Some(b) = app_config.cloud.enabled {
                    if b {
                        handle_upload(p)
                    } else {
                        info!("Upload disabled -- video retained at {}", &p);
                    }
                }
            }
            Err(e) => error!("Video writing failed: {}", e),
        }
    });

    video_tx
}

fn handle_upload(path: String) -> () {
    match Runtime::new().unwrap().block_on(upload::upload_file(&path)) {
        Ok(_) => {
            debug!("Deleting file {}", &path);
            fs::remove_file(path).unwrap();
        }
        Err(e) => {
            error!("File upload failed: {}", e);
            warn!(
                "Skipping deletion due to upload failure; video retained at {}",
                &path
            );
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

pub fn init_encoder<'a>(
    width: u32,
    height: u32,
    octx: &mut Output,
    fps: i16,
    set_global_hdr: bool,
) -> Video {
    let config = config::load_config(None);

    ffmpeg::util::log::set_level(config.log_level.ffmpeg());
    ffmpeg::init().unwrap();

    // let x264_opts = parse_opts("enable-debug=3".to_string());
    let x264_opts = parse_opts("".to_string());

    let mut encoder = octx
        .add_stream(codec::encoder::find(codec::Id::H264))
        .unwrap()
        .codec()
        .encoder()
        .video()
        .unwrap();

    encoder.set_width(width);
    encoder.set_height(height);
    encoder.set_format(VideoProc::video_format());
    encoder.set_time_base(Rational::new(1, fps.into()));

    if set_global_hdr {
        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        if global_header {
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }
    }

    encoder.open_with(x264_opts).expect("couldn't open encoder");
    // Reassigned due to move in line above:
    // Getting reference to stream here rather than using one from above to avoid multiple borrows of octx:
    encoder = octx
        .stream_mut(0)
        .unwrap()
        .codec()
        .encoder()
        .video()
        .unwrap();

    encoder
}
