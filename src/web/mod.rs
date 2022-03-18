use crate::config;
use crate::file_source;
use crate::frame::Frame;
use crate::video::{rtc_track::RTCTrack, VideoRTCStream};

mod api;

use log::error;
use rocket::fs::FileServer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver as AsyncReceiver;
use tokio::task::JoinHandle;

pub async fn start(
    receivers: Vec<AsyncReceiver<Arc<Frame>>>,
    cameras: Vec<config::CameraConfig>,
) -> () {
    let (streams, threads) = start_async(receivers, cameras).await;
    if let Err(e) = rocket::build()
        .mount(
            "/api",
            routes![
                api::get_stream,
                api::get_streams_list,
                api::get_videos,
                api::get_video_by_name,
            ],
        )
        .mount("/", FileServer::from("web"))
        .manage(streams)
        .manage(file_source::load())
        .manage(config::load_config(None))
        .launch()
        .await
    {
        error!("Failed to launch rocket: {}", e);
    }
}

async fn start_async(
    receivers: Vec<AsyncReceiver<Arc<Frame>>>,
    cameras: Vec<config::CameraConfig>,
) -> (HashMap<String, Arc<RTCTrack>>, Vec<JoinHandle<()>>) {
    receivers
        .into_iter()
        .zip(cameras.into_iter())
        .map(
            |(mut rx, camera)| -> (String, Arc<RTCTrack>, JoinHandle<()>) {
                let label = camera.label.clone();
                let stream = Arc::new(VideoRTCStream::new(camera));
                let track = stream.track();

                let thread = tokio::spawn(async move {
                    let f = rx.recv().await.unwrap();
                    stream.start(f.width(), f.height(), rx).await;
                });

                (label, track, thread)
            },
        )
        .fold(
            (HashMap::new(), Vec::new()),
            |mut acc, (label, track, thread)| {
                acc.0.insert(label, track);
                acc.1.push(thread);
                acc
            },
        )
}
