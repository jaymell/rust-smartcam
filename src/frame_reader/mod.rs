mod opencv;
mod rtsp;
mod v4l;

pub use self::rtsp::RTSPFrameReader;
pub use self::v4l::V4LFrameReader;
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
    ) -> Result<()>;
}

pub fn start_frame_reader(
    cam_type: &str,
    senders: Vec<Sender<Arc<Frame>>>,
    web_tx: Option<AsyncSender<Arc<Frame>>>,
    source: Option<&str>,
) -> Result<()> {
    match cam_type {
        "rtsp" => {
            let frame_reader = RTSPFrameReader {};
            frame_reader.read_frames(senders, web_tx, source);
        }
        "v4l" => {
            let frame_reader = V4LFrameReader {};
            frame_reader.read_frames(senders, web_tx, source);
        }
        _ => {
            panic!("Unknown camera type");
        }
    };

    Ok(())
}
