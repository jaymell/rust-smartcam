

window.startSession = async (s, pc) => {

  let sd = btoa(JSON.stringify(pc.localDescription));
  console.log("Posting ", sd);
  let remote = await fetch(`/api/streams/${s}`,
    {
      method: 'POST',
      body: sd,
      headers: { 'Content-Type': 'text/plain' }
    }
  );
  let b = await remote.text();
  console.log(b);
  try {
    pc.setRemoteDescription(new RTCSessionDescription(JSON.parse(atob(b))))
  } catch (e) {
    console.error(e);
  }

};


function initiatePeerConnection(streamName) {

  const buttonDiv = document.getElementById("buttonHolder");
  const buttonId = `button-${streamName}`;

  // let pc = new RTCPeerConnection();

  let pc = new RTCPeerConnection({
    iceServers: [
      {
        // FIXME -- configuration:
        urls: 'stun:stun.l.google.com:19302'
        // urls: 'stun:stun.voipinfocenter.com:3478'
      }
    ]
  });


  pc.ontrack = event => {
    var el = document.createElement(event.track.kind)
    el.srcObject = event.streams[0]
    el.autoplay = true
    el.controls = true

    buttonDiv.appendChild(el);
  };

  // Offer to receive 1 audio, and 1 video track
  pc.addTransceiver('video', {'direction': 'sendrecv'})
  pc.addTransceiver('audio', {'direction': 'sendrecv'})

  pc.createOffer()
    .then(d => {
      console.log("Setting local description: ", d);
      pc.setLocalDescription(d);
    })
    .catch(console.error);


  pc.oniceconnectionstatechange = e => {
    console.log("connection state change: ", JSON.stringify(e));
    const state = pc.iceConnectionState;
    if (state == 'disconnected' || state == 'failed') {
      alert("Disconnected");
      location.reload();
    }
  };


  pc.onicecandidate = e => {
    if (e.candidate === null) {
      console.log("ICE gathering complete");
      console.log('local session description: ', JSON.stringify(pc.localDescription));
      return;
    }
    console.log("onicecandidate: ", e.candidate);
    // let cand = btoa(JSON.stringify(e.candidate));
    // let remote = fetch(`/api/streams/candidate`, {
    //   method: 'POST',
    //   body: cand,
    //   headers: { 'Content-Type': 'text/plain' }
    // });
    // let b = await remote.text();
  };

  const b = document.createElement('button');
  b.id = buttonId;
  b.innerHTML = streamName;
  b.onclick = () => window.startSession(streamName, pc);
  buttonDiv.appendChild(b);
}


function createVideoDiv(label, video) {
  const ctr = document.getElementById("remoteVideos");
  const videoTag = document.createElement("video");
  videoTag.width = 320;
  videoTag.height = 240;
  videoTag.controls = true;
  videoTag.preload = "metadata";

  const source = document.createElement("source");
  source.src = `/api/videos/${label}/${video}`;
  source.type = "video/mp4";

  const div = document.createElement("div");
  div.appendChild(videoTag);
  videoTag.appendChild(source);
  ctr.appendChild(div);
}

document.addEventListener('DOMContentLoaded', async function(event) {

  const streams = await (await fetch('/api/streams')).json();

  console.log("streams: ", streams);

  streams.forEach(it => initiatePeerConnection(it));

  const fetchArray = await Promise.all(streams.map(it => fetch(`/api/videos/${it}`)));

  console.log("fetchArray: ", fetchArray);

  const vidsArray = await Promise.all(fetchArray.map(async(it) => await it.json()));

  vidsArray.forEach(vidArray =>
    vidArray.forEach(vid => createVideoDiv("Frontdoor", vid.file_name)));

});
