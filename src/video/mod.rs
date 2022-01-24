pub mod rtc_track;

use crate::config;
use crate::frame::{Frame, VideoFrame};
use crate::upload;
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

pub fn start_video_writer(
    start_time: DateTime<Utc>,
    width: u32,
    height: u32,
) -> Sender<VideoFrame> {
    let (video_tx, video_rx) = mpsc::channel::<VideoFrame>();

    thread::spawn(move || -> () {
        let app_config = config::load_config(None);
        let mut video_frame_proc = VideoFileWriter::new(start_time, width, height, None);
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

/// Return frame pts and duration in milliseconds
fn calc_frame_duration(
    previous_frame_time: Option<DateTime<Utc>>,
    previous_pts: Option<i64>,
    ts: DateTime<Utc>,
    fps: i16,
) -> (Option<i64>, Option<i64>) {
    match previous_frame_time {
        Some(t) => {
            match previous_pts {
                Some(p) => {
                    let delta = (ts - t).num_milliseconds();
                    // let result = ((delta as f64) / self.fps as f64).round() as i64 + p;
                    // let result = (self.frame_count as f64 / self.fps as f64).round() as i64;
                    // let result = p + 300;
                    let result = ((delta as f64 / 1000.0) * fps as f64 + p as f64).round() as i64;
                    trace!("PTS {:?}", result);
                    (Some(result), Some(delta))
                }
                None => {
                    warn!("Last PTS not found.");
                    (None, None)
                }
            }
        }
        None => (None, None),
    }
}

pub fn init_encoder<'a>(
    width: u32,
    height: u32,
    octx: &mut Output,
    fps: i16,
    set_global_hdr: bool,
) -> Video {
    ffmpeg::util::log::set_level(Level::Info);
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

struct VideoProc {
    fps: i16,
    previous_frame_time: Option<DateTime<Utc>>,
    previous_pts: Option<i64>,
    frame_count: i64,
    octx: Output,
    pub encoder: Video,
}

impl VideoProc {
    pub fn new(fps: i16, octx: Output, encoder: Video) -> Self {
        Self {
            fps: fps,
            previous_frame_time: None,
            previous_pts: None,
            frame_count: 0,
            octx: octx,
            encoder: encoder,
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

    pub fn video_format() -> Pixel {
        Pixel::YUV420P
        // Pixel::BGR24
    }

    fn fps(&self) -> i16 {
        self.fps
    }

    /// convert frame to ffmpeg frame and write to encoder, return frame duration
    pub fn process_frame(&mut self, frame: Arc<Frame>) -> Option<i64> {
        unsafe {
            // unsafe:
            let mut dst = av_frame_alloc();
            (*dst).width = frame.width() as _;
            (*dst).height = frame.height() as _;
            (*dst).format =
                mem::transmute::<AVPixelFormat, c_int>(VideoProc::image_format_raw().into());
            // unsafe:
            av_frame_get_buffer(dst, 32);
            // unsafe:
            avpicture_fill(
                dst as *mut AVPicture,
                frame.img().datastart(),
                AVPixelFormat::from(VideoProc::image_format()),
                self.encoder.width() as _,
                self.encoder.height() as _,
            );
            trace!("Buffer size is {:?}", (*(*dst).buf[0]).size);
            let mut video_frame = frame::Video::wrap(dst);
            video_frame.set_width(frame.width());
            video_frame.set_height(frame.height());
            video_frame.set_format(VideoProc::image_format());

            let mut converted = frame::Video::empty();
            converted.set_width(frame.width());
            converted.set_height(frame.height());
            converted.set_format(VideoProc::video_format());

            video_frame
                .converter(VideoProc::video_format())
                .unwrap()
                .run(&video_frame, &mut converted)
                .unwrap();

            let (pts, duration_ms) = calc_frame_duration(
                self.previous_frame_time,
                self.previous_pts,
                frame.time(),
                self.fps,
            );
            let pts = pts.unwrap_or(0);
            converted.set_pts(Some(pts));
            self.encoder.send_frame(&converted).unwrap();

            self.previous_frame_time = Some(frame.time());
            self.previous_pts = Some(pts);
            self.frame_count = self.frame_count + 1;

            duration_ms
        }
    }
}

struct VideoFileWriter {
    start_time: DateTime<Utc>,
    video_proc: VideoProc,
    path: PathBuf,
}

impl VideoFileWriter {
    // FIXME -- return result:
    pub fn new(start_time: DateTime<Utc>, width: u32, height: u32, fps: Option<i16>) -> Self {
        let f = format!("/tmp/{}.mkv", start_time.format("%+"));
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
        }
    }

    fn fps(&self) -> i16 {
        self.video_proc.fps()
    }

    fn close_file(&mut self) {
        self.video_proc.encoder.send_eof().unwrap();
        self.video_proc.octx.write_trailer().unwrap();
    }

    pub fn receive_file(
        &mut self,
        receiver: Receiver<VideoFrame>,
    ) -> Result<String, Box<dyn Error>> {
        loop {
            let video_frame = receiver.recv().unwrap();
            let frame = video_frame.frame;
            let _ = self.video_proc.process_frame(frame);
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
                .write_interleaved(&mut self.video_proc.octx)
                .unwrap();
        }
        trace!("Finished writing packets...");
    }
}

pub struct VideoRTCStream {
    track: Arc<RTCTrack>,
    camera: config::CameraConfig,
}

impl VideoRTCStream {
    pub fn new(camera: config::CameraConfig) -> Self {
        // Create a video track
        let video_track = Arc::new(RTCTrack::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            camera.label.clone(),
        ));

        Self {
            track: video_track,
            camera: camera,
        }
    }

    pub async fn start(
        &self,
        width: u32,
        height: u32,
        fps: Option<i16>,
        mut rx: AsyncReceiver<Arc<Frame>>,
    ) {
        unsafe {
            let fps = fps.unwrap_or(1000);
            // unsafe:
            let mut octx = format::context::output::Output::wrap(avformat_alloc_context());
            let encoder = init_encoder(width, height, &mut octx, fps, false);

            let mut video_proc = VideoProc::new(fps, octx, encoder);

            // It is important to use a time.Ticker instead of time.Sleep because
            // * avoids accumulating skew, just calling time.Sleep didn't compensate for the time spent parsing the data
            // * works around latency issues with Sleep
            let mut ticker = tokio::time::interval(
                Duration::milliseconds((fps as f64 / 1000 as f64) as _)
                    .to_std()
                    .unwrap(),
            );

            debug!("Receiving stream {}", self.camera.label);
            while let Some(frame) = rx.recv().await {
                let num_conns = *self.track.num_conns.lock().unwrap();
                if num_conns == 0 {
                    trace!("No connections -- continuing");
                    continue;
                } else {
                    trace!("{} active connection", num_conns);
                }

                trace!("Writing frame to encoder");
                let duration_ms = video_proc.process_frame(frame).unwrap_or(1);
                let ost_index = 0;
                let mut encoded = Packet::empty();
                encoded.set_stream(ost_index);
                while video_proc.encoder.receive_packet(&mut encoded).is_ok() {
                    trace!("Getting bytes from encoder");
                    if let Err(e) = &self
                        .track
                        .write_sample(&Sample {
                            data: Bytes::copy_from_slice(
                                encoded.data().expect("Failed to get encoded data"),
                            ),
                            duration: Duration::milliseconds(duration_ms).to_std().unwrap(),
                            ..Default::default()
                        })
                        .await
                    {
                        error!("Failed to write to video_track: {}", e);
                    };
                    let _ = ticker.tick().await;
                }
            }

            error!("Video stream loop terminated");
        }
    }

    pub fn track(&self) -> Arc<RTCTrack> {
        Arc::clone(&self.track)
    }
}
