let pc = new RTCPeerConnection({
    iceServers: [
        {
            urls: 'stun:stun.comrex.com:3478'
        }
    ]
})

window.onload = async function () {
    var video = document.querySelector("#webcamVideo");

    try {
            if (navigator.mediaDevices.getUserMedia) {
            var stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
            stream.getTracks().forEach((track) =>
                pc.addTrack(track, stream));
            video.srcObject = stream;
        }
    } catch(err) {
        console.log("Something went wrong!" + err);
    };
}

let log = msg => {
    document.getElementById('div').innerHTML += msg + '<br>'
}

pc.oniceconnectionstatechange = e => log(pc.iceConnectionState)
pc.onicecandidate = e => log(e)

window.startSession = () => {
    let sd = document.getElementById('remoteSessionDescription').value
    console.log(sd)
    if (sd === ' ') {
        return alert('Session Description must not be empty')
    }

    try {
        pc.setRemoteDescription(new RTCSessionDescription({type: 'offer', sdp: atob(sd)}))
    } catch (e) {
        alert(e)
    }

    pc.createAnswer().then(async d => {
        await pc.setLocalDescription(d)
        document.getElementById('localSessionDescription').value = btoa(pc.localDescription.sdp)
    }).catch(log)
}
