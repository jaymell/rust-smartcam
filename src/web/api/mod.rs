use crate::config::Config;
use crate::file_source;
use crate::video::rtc_track::RTCTrack;

use log::{debug, error};
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::track::track_local::TrackLocal;

#[get("/videos/<label>")]
pub(crate) async fn get_videos(
    label: String,
    fs: &State<Arc<dyn file_source::FileSource + Send + Sync>>,
) -> Json<Vec<file_source::VideoFile>> {
    Json(fs.list_files_by_label(&label).await.unwrap())
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
