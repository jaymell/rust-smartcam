

window.startSession = async (s, pc) => {

  let sd = btoa(JSON.stringify(pc.localDescription));
  let remote = await fetch(`/api/streams/${s}`, { method: 'POST', body: sd, headers: { 'Accept': 'text/plain' } });
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

  let pc = new RTCPeerConnection({
    iceServers: [
      {
        // FIXME -- configuration:
        urls: 'stun:stun.l.google.com:19302'
      }
    ]
  });
  pc.ontrack = function (event) {
    var el = document.createElement(event.track.kind)
    el.srcObject = event.streams[0]
    el.autoplay = true
    el.controls = true

    buttonDiv.appendChild(el);
  };
  // Offer to receive 1 audio, and 1 video track
  pc.addTransceiver('video', {'direction': 'sendrecv'})
  pc.addTransceiver('audio', {'direction': 'sendrecv'})
  pc.createOffer().then(d => pc.setLocalDescription(d)).catch(console.log);
  pc.oniceconnectionstatechange = e => console.log(pc.iceConnectionState);
  pc.onicecandidate = event => {
    if (event.candidate === null) {
      console.log('local session description: ', JSON.stringify(pc.localDescription));
    }
  };

  const b = document.createElement('button');
  b.id = buttonId;
  b.innerHTML = streamName;
  b.onclick = () => window.startSession(streamName, pc);
  buttonDiv.appendChild(b);

}


document.addEventListener('DOMContentLoaded', function(event) {

  fetch('/api/streams')
    .then(async(it) => (await it.json()) )
    .then(it => it.forEach(it => initiatePeerConnection(it)) )
    .catch(e => { console.error(e); });

});