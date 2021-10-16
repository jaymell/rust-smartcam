mod opencv;
mod rtsp;
mod v4l;

pub use crate::frame_reader::opencv::start as start_opencv;
pub use crate::frame_reader::rtsp::start as start_rtsp;
pub use crate::frame_reader::v4l::start as start_v4l;
