use crate::frame::Frame;
use chrono::{DateTime, Utc};
use ffmpeg::{
    codec::encoder::video::Video, format::context::output::Output, format::Pixel, frame,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next as ffs;
use ffs::{av_frame_alloc, av_frame_get_buffer, avpicture_fill, AVPicture, AVPixelFormat};
use libc::c_int;
use log::{trace, warn};
use opencv::core::prelude::MatTrait;
use std::mem;
use std::sync::Arc;

pub struct VideoProc {
    fps: i32,
    previous_frame_time: Option<DateTime<Utc>>,
    previous_pts: Option<i64>,
    frame_count: i64,
    octx: Output,
    pub encoder: Video,
}

impl VideoProc {
    pub fn new(fps: i32, octx: Output, encoder: Video) -> Self {
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

    pub fn octx(&self) -> &Output {
        &self.octx
    }

    pub fn octx_mut(&mut self) -> &mut Output {
        &mut self.octx
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

/// Return frame pts and duration in milliseconds
fn calc_frame_duration(
    previous_frame_time: Option<DateTime<Utc>>,
    previous_pts: Option<i64>,
    ts: DateTime<Utc>,
    fps: i32,
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
