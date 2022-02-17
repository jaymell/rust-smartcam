use crate::config::Config;
use crate::frame::{Frame, VideoFrame};
use crate::repository::{VideoFile, VideoRepository};
use crate::upload;
use crate::video::{init_encoder, rtc_track::RTCTrack, VideoRTCStream};

use chrono;
use chrono::{DateTime, Duration, Utc};
use ffmpeg::{
    codec, codec::encoder::video::Video, format, format::Pixel, frame, util::rational::Rational,
    Dictionary, Packet,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_sys_next as ffs;
use ffs::{
    av_frame_alloc, av_frame_get_buffer, av_guess_format, avformat_alloc_context,
    avio_alloc_context, avpicture_fill, AVPicture, AVPixelFormat, AVFMT_FLAG_CUSTOM_IO,
};
use futures::join;
use libc::c_int;
use log::{debug, error, info, trace, warn};
use opencv::core::prelude::MatTrait;
use rocket::fs::{FileServer, NamedFile};
use rocket::http::Status;
use rocket::response::{content, status, stream::TextStream};
use rocket::serde::json::Json;
use rocket::State;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread;
use std::{fs, mem, ptr};
use tokio::sync::mpsc::{channel as async_channel, Receiver as AsyncReceiver};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::{runtime::Runtime, task};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::h264_writer::H264Writer;
use webrtc::media::io::ogg_writer::OggWriter;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_remote::TrackRemote;

use webrtc::ice_transport::ice_candidate::RTCIceCandidate;

use tokio_stream::StreamExt;

#[get("/videos/<label>")]
pub(crate) async fn get_videos_list(
    label: String,
    fs: &State<Arc<dyn VideoRepository + Send + Sync>>,
) -> Json<Vec<VideoFile>> {
    let s = fs.stream_files_by_label(label.clone()).await;
    Json(s.collect().await)
}

#[get("/videos/<label>/<video>")]
pub(crate) async fn get_video_by_name(
    label: String,
    video: PathBuf,
    state: &State<HashMap<String, Arc<RTCTrack>>>,
    config: &State<Arc<Config>>,
) -> Option<NamedFile> {
    NamedFile::open(Path::new(&config.storage.path).join(video))
        .await
        .ok()
}

#[get("/streams")]
pub(crate) async fn get_streams_list(
    state: &State<HashMap<String, Arc<RTCTrack>>>,
) -> Json<Vec<String>> {
    Json(state.keys().map(|s: &String| s.clone()).collect())
}

#[post("/streams/<label>", data = "<offer>")]
pub(crate) async fn get_stream(
    label: String,
    offer: String,
    state: &State<HashMap<String, Arc<RTCTrack>>>,
) -> Result<String, Status> {
    let video_track = state.get(&label);
    if let None = video_track {
        // 404
        return Err(Status::NotFound);
    }
    let video_track = video_track.unwrap();

    let mut m = MediaEngine::default();
    m.register_default_codecs().unwrap();

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)
        .await
        .unwrap();

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            // FIXME -- configure:
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());

    // Add this newly created track to the PeerConnection
    let rtp_sender = peer_connection
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .unwrap();

    let (discon_tx, mut discon_rx) = tokio::sync::mpsc::channel::<bool>(1);

    let discon_tx = Arc::new(discon_tx);
    peer_connection
        .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            debug!("Peer Connection State has changed: {}", s);
            if s == RTCPeerConnectionState::Disconnected || s == RTCPeerConnectionState::Failed {
                let discon_tx = Arc::clone(&discon_tx);
                tokio::spawn(async move {
                    if let Err(e) = discon_tx.send(true).await {
                        error!("WTF: {}", e);
                    }
                });
            }
            Box::pin(async {})
        }))
        .await;

    // Read incoming RTCP packets
    // tokio::spawn(async move {
    //     let mut rtcp_buf = vec![0u8; 1500];
    //     while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
    //     Result::<()>::Ok(())
    // });

    let offer = base64::decode(&offer).unwrap();
    let offer = String::from_utf8(offer).unwrap();
    debug!("Received offer {}", offer);
    let offer = serde_json::from_str::<RTCSessionDescription>(&offer.as_str()).unwrap();

    // Set the remote SessionDescription
    peer_connection.set_remote_description(offer).await.unwrap();

    // Create an answer
    let answer = peer_connection.create_answer(None).await.unwrap();

    // Create channel that is blocked until ICE Gathering is complete
    let mut gather_complete = peer_connection.gathering_complete_promise().await;

    // Sets the LocalDescription, and starts our UDP listeners
    peer_connection.set_local_description(answer).await.unwrap();

    // Block until ICE Gathering is complete, disabling trickle ICE
    let _ = gather_complete.recv().await;

    match peer_connection.local_description().await {
        Some(local_desc) => {
            let json_str = serde_json::to_string(&local_desc).unwrap();
            let mut rtp_sender = Some(rtp_sender);
            tokio::spawn(async move {
                discon_rx.recv().await;
                peer_connection
                    .remove_track(&mut rtp_sender.take().unwrap())
                    .await
                    .unwrap();
                peer_connection.close().await.unwrap();
            });
            Ok(base64::encode(&json_str))
        }
        None => Err(Status::InternalServerError),
    }
}
