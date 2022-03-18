use super::init_encoder;
use super::VideoProc;
use crate::config;
use crate::frame::VideoFrame;
use crate::FileSourceType;

use chrono;
use chrono::{DateTime, Utc};
use ffmpeg::{format, util::rational::Rational, Packet};
use ffmpeg_next as ffmpeg;

use log::{debug, trace};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
pub struct VideoFileWriter {
    video_proc: VideoProc,
    path: PathBuf,
    fps: i32,
    _temp_path: &'static str,
}

impl VideoFileWriter {
    // FIXME -- return result:
    pub fn new(label: String, start_time: DateTime<Utc>, width: u32, height: u32) -> Self {
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
        let fps = 90000;
        let mut octx = format::output(&p).unwrap();
        let encoder = init_encoder(width, height, &mut octx, fps, true);

        format::context::output::dump(&octx, 0, Some(&f));
        octx.write_header().unwrap();

        Self {
            video_proc: VideoProc::new(fps, octx, encoder),
            path: p,
            fps,
            _temp_path: temp_path,
        }
    }

    fn close_file(&mut self) {
        self.video_proc.encoder.send_eof().unwrap();
        self.video_proc.octx_mut().write_trailer().unwrap();
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
        let source_tb = Rational::new(1, self.fps);
        let stream_tb = self
            .video_proc
            .octx()
            .stream(ost_index)
            .unwrap()
            .time_base();
        while self.video_proc.encoder.receive_packet(&mut encoded).is_ok() {
            trace!("Writing packets...");
            encoded.set_stream(ost_index);
            encoded.rescale_ts(source_tb, stream_tb);
            encoded
                .write_interleaved(&mut self.video_proc.octx_mut())
                .unwrap();
        }
        trace!("Finished writing packets...");
    }
}
