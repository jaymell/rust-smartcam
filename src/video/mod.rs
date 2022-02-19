mod file_writer;
mod rtc_stream;
pub mod rtc_track;
mod video_proc;

use crate::config;
use crate::config::CameraConfig;
use crate::frame::VideoFrame;
use crate::upload;
use chrono;
use chrono::{DateTime, Utc};
use ffmpeg::{
    codec, codec::encoder::video::Video, format, format::context::output::Output,
    util::rational::Rational, Dictionary,
};
use ffmpeg_next as ffmpeg;
use log::{debug, error, info, warn};
use rtc_track::RTCTrack;
use std::fs;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::runtime::Runtime;

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

    ffmpeg::util::log::set_level(config.ffmpeg_level.ffmpeg());
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
