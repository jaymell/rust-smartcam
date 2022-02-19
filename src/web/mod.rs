mod api;
use crate::config;
use crate::frame::Frame;
use crate::repository;
use crate::video::{rtc_track::RTCTrack, VideoRTCStream};
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
    rocket::build()
        .mount(
            "/api",
            routes![
                api::get_stream,
                api::get_streams_list,
                api::get_videos_list,
                api::get_video_by_name,
            ],
        )
        .mount("/", FileServer::from("web"))
        .manage(streams)
        .manage(repository::load())
        .manage(config::load_config(None))
        .launch()
        .await
        .expect("Failed to start Rocket");
    // for t in threads {
    //     t.await;
    // }
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
                    stream.start(f.width(), f.height(), None, rx).await;
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
