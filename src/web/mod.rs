use crate::config;
use crate::frame::{Frame, VideoFrame};
use crate::file_source;
use crate::upload;
use crate::video::{init_encoder, rtc_track::RTCTrack, VideoRTCStream};

mod api;

use futures::join;
use log::{debug, error, info, trace, warn};
use rocket::fs::FileServer;
use rocket::response::{content, status};
use rocket::serde::json::Json;
use rocket::State;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread;
use std::{fs, mem, ptr};
use tokio::sync::mpsc::{channel as async_channel, Receiver as AsyncReceiver};
use tokio::task::JoinHandle;


pub async fn start(
    receivers: Vec<AsyncReceiver<Arc<Frame>>>,
    cameras: Vec<config::CameraConfig>,
) -> () {
    let (streams, threads) = start_async(receivers, cameras).await;
    rocket::build()
        .mount("/api", routes![api::get_stream, api::get_streams_list, api::get_videos])
        .mount("/", FileServer::from("web"))
        .manage(streams)
        .manage(file_source::load())
        .launch()
        .await;
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
