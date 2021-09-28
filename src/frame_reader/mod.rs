mod frame_reader_opencv;
mod frame_reader_v4l;
mod rtsp;

pub use frame_reader_opencv::start as start_opencv;
pub use frame_reader_v4l::start as start_v4l;
pub use rtsp::start as start_rtsp;
