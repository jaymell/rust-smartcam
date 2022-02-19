use std::any::Any;
use std::sync::{Arc, Mutex};
use webrtc::error::Result;
use webrtc::media::Sample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecParameters, RTPCodecType};
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::{TrackLocal, TrackLocalContext};

/// Implementation of TrackLocalStaticSample that tracks
/// total number of active bindings
pub struct RTCTrack {
    pub track: TrackLocalStaticSample,
    pub num_conns: Arc<Mutex<u32>>,
}

impl RTCTrack {
    pub fn new(codec: RTCRtpCodecCapability, id: String, stream_id: String) -> Self {
        let track = TrackLocalStaticSample::new(codec, id, stream_id);

        RTCTrack {
            track: track,
            num_conns: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn write_sample(&self, sample: &Sample) -> Result<()> {
        self.track.write_sample(sample).await
    }
}

#[async_trait]
impl TrackLocal for RTCTrack {
    async fn bind(&self, t: &TrackLocalContext) -> Result<RTCRtpCodecParameters> {
        let b = self.track.bind(t).await;
        let mut num_conns = self.num_conns.lock().unwrap();
        *num_conns += 1;
        debug!(
            "Binding connection for stream {} -- total connections {}",
            self.stream_id(),
            *num_conns
        );
        b
    }

    async fn unbind(&self, t: &TrackLocalContext) -> Result<()> {
        let u = self.track.unbind(t).await;
        let mut num_conns = self.num_conns.lock().unwrap();
        *num_conns -= 1;
        debug!(
            "Removing connection for stream {} -- total connections {}",
            self.stream_id(),
            *num_conns
        );
        u
    }

    fn id(&self) -> &str {
        self.track.id()
    }

    fn stream_id(&self) -> &str {
        self.track.stream_id()
    }

    fn kind(&self) -> RTPCodecType {
        self.track.kind()
    }

    fn as_any(&self) -> &dyn Any {
        self.track.as_any()
    }
}
