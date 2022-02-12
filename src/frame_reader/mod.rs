mod opencv;
mod rtsp;
mod v4l;

pub use self::rtsp::RTSPFrameReader;
pub use self::v4l::V4LFrameReader;
use crate::config::CameraConfig;
use crate::frame::Frame;
use anyhow::Result;
use std::sync::{mpsc::Sender, Arc};
use tokio::sync::mpsc::Sender as AsyncSender;

pub trait FrameReader {
    fn read_frames(
        &self,
        senders: Vec<Sender<Arc<Frame>>>,
        web_tx: Option<AsyncSender<Arc<Frame>>>,
        source: Option<&str>,
    );
}

pub fn start_frame_reader(
    camera: Arc<CameraConfig>,
    senders: Vec<Sender<Arc<Frame>>>,
    web_tx: Option<AsyncSender<Arc<Frame>>>,
) -> Result<()> {
    match camera.camera_type.as_str() {
        "rtsp" => {
            let frame_reader = RTSPFrameReader {};
            frame_reader.read_frames(senders, web_tx, camera.source.as_deref());
        }
        "v4l" => {
            let frame_reader = V4LFrameReader {};
            frame_reader.read_frames(senders, web_tx, camera.source.as_deref());
        }
        _ => {
            panic!("Unknown camera type");
        }
    };

    Ok(())
}
