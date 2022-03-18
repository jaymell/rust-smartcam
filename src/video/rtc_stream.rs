use super::init_encoder;
use super::{RTCTrack, VideoProc};
use crate::config;
use crate::frame::Frame;

use bytes::Bytes;
use chrono;
use chrono::Duration;
use ffmpeg::{format, Packet};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next as ffs;
use ffs::avformat_alloc_context;
use log::{debug, error, trace};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver as AsyncReceiver;
use webrtc::api::media_engine::MIME_TYPE_H264;
use webrtc::media::Sample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;

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

    pub async fn start(&self, width: u32, height: u32, mut rx: AsyncReceiver<Arc<Frame>>) {
        let fps = 90000;
        // unsafe:
        let mut octx = unsafe { format::context::output::Output::wrap(avformat_alloc_context()) };
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

    pub fn track(&self) -> Arc<RTCTrack> {
        Arc::clone(&self.track)
    }
}
