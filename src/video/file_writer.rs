use super::init_encoder;
use super::VideoProc;
use crate::config;

use crate::frame::VideoFrame;

use crate::FileSourceType;

use chrono;
use chrono::{DateTime, Utc};
use ffmpeg::{format, Packet};
use ffmpeg_next as ffmpeg;

use log::{debug, trace};

use std::error::Error;

use std::path::{Path, PathBuf};

use std::sync::mpsc::Receiver;

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
            trace!("Frame duration: {:?}", frame_duration);
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
