use std::str::FromStr;
use gst::{Element};
use gst::prelude::*;
use anyhow::Result;
use clipboard::{ClipboardContext, ClipboardProvider};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use webrtcredux::webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtcredux::webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtcredux::webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtcredux::{RTCIceConnectionState, RTCSdpType};
use tokio::runtime::Handle;
use webrtcredux::sdp::LineEnding::LF;

use webrtcredux::webrtcredux::{
    sdp::{SDP},
    RTCIceServer, WebRtcRedux,
};

pub fn must_read_stdin() -> Result<String> {
    let mut line = String::new();

    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}

pub fn decode(s: &str) -> Result<String> {
    let b = base64::decode(s)?;

    let s = String::from_utf8(b)?;
    Ok(s)
}

pub fn encode(s: &str) -> Result<String> {
    Ok(base64::encode(s))
}

async fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    stdout.write_all(b"\nPress Enter to continue...").await.unwrap();
    stdout.flush().await.unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).await.unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    gst::init().unwrap();
    webrtcredux::plugin_register_static().unwrap();

    let pipeline = gst::Pipeline::new(None);

    let webrtcredux = WebRtcRedux::default();

    webrtcredux.set_tokio_runtime(Handle::current());

    webrtcredux.add_ice_servers(vec![RTCIceServer {
        urls: vec!["stun:stun.comrex.com:3478".to_string()],
        ..Default::default()
    }]);

    pipeline
        .add(webrtcredux.upcast_ref::<gst::Element>())
        .expect("Failed to add webrtcredux to the pipeline");

    // VIDEO
    let video_depay = gst::ElementFactory::make("rtpvp8depay", None)?;
    let video_decoder = gst::ElementFactory::make("avdec_vp8", None)?;
    let video_convert = gst::ElementFactory::make("videoconvert", None)?;
    let video_sink = gst::ElementFactory::make("autovideosink", None)?;

    pipeline.add_many(&[&video_depay, &video_decoder, &video_convert, &video_sink])?;

    Element::link_many(&[&video_depay, &video_decoder, &video_convert, &video_sink])?;

    // AUDIO
    let audio_depay = gst::ElementFactory::make("rtpopusdepay", None)?;
    let audio_decoder = gst::ElementFactory::make("opusdec", None)?;
    let audio_convert = gst::ElementFactory::make("audioconvert", None)?;
    let audio_sink = gst::ElementFactory::make("autoaudiosink", None)?;

    pipeline.add_many(&[&audio_depay, &audio_decoder, &audio_convert, &audio_sink])?;

    Element::link_many(&[&audio_depay, &audio_decoder, &audio_convert, &audio_sink])?;

    // Sources get linked to depay* elements on pad-added signal, when sometimes src_%u-pads are announced

    // Start pipeline to be able to get an internal reference to PeerConnection inside webrtcredux
    pipeline.set_state(gst::State::Playing)?;

    // Add expected transceivers
    webrtcredux.add_transceiver(
        RTPCodecType::Audio,
        &[RTCRtpTransceiverInit {
            direction: RTCRtpTransceiverDirection::Recvonly,
            send_encodings: vec![],
        }]
    ).await?;

    webrtcredux.add_transceiver(
        RTPCodecType::Video,
        &[RTCRtpTransceiverInit {
            direction: RTCRtpTransceiverDirection::Recvonly,
            send_encodings: vec![],
        }]
    ).await?;

    // Create local SDP offer
    let offer = webrtcredux.create_offer(None).await?;
    webrtcredux.set_local_description(&offer, RTCSdpType::Offer).await?;

    let mut clipboard_handle = ClipboardContext::new().expect("Failed to create clipboard context");

    // Copy local SDP offer to clipboard
    if let Ok(Some(local_desc)) = webrtcredux.local_description().await {
        let b64 = base64::encode(local_desc.to_string(LF));
        clipboard_handle.set_contents(b64.clone()).expect("Failed to set clipboard contents");
        println!("Base64 Session Description for the browser copied to the cliboard", );
        println!("{}", b64);
    } else {
        println!("generate local_description failed!");
    }

    pause().await;

    // Set remote SDP answer
    let line = clipboard_handle.get_contents().expect("Failed to get clipboard contents");
    let sdp_answer_from_b64 = decode(line.as_str())?;
    let answer = SDP::from_str(&sdp_answer_from_b64).expect("Failed to parse SDP");
    webrtcredux.set_remote_description(&answer, RTCSdpType::Answer).await?;

    webrtcredux.on_peer_connection_state_change(Box::new(|state| {
        println!("Peer connection state has changed {}", state);

        Box::pin(async {})
    })).await?;

    webrtcredux.on_ice_connection_state_change(Box::new(move |connection_state: RTCIceConnectionState| {
        println!("Connection State has changed {}", connection_state);

        Box::pin(async {})
    }))
        .await?;

    // Directly hook into `on_track` to be notified about incoming stream
    webrtcredux.on_track(Box::new(move |remote_track, _rtp_receiver| {
        println!("On track: {:?} {:?}", remote_track, _rtp_receiver);

        Box::pin(async {})
    }))
        .await?;

    // Listen for incoming streams to bind to depay*
    webrtcredux.connect_pad_added(move |_, pad| {
        let pad_name = pad.name();
        println!("Pad added {} direction={:?} caps={:?}", pad_name, pad.direction(), pad.caps());
        let depay_element = match pad_name.as_str() {
            "src_0" => {
                &video_depay
            },
            "src_1" => {
                &audio_depay
            },
            _ => {
                unreachable!()
            }
        };
        let depay_sink = &depay_element
            .static_pad("sink")
            .expect("Failed to get sink from depay element");
        pad.link(depay_sink)
            .expect("Failed to link src pad to depay_sink");
    });

    let mut gather_complete = webrtcredux.gathering_complete_promise().await?;
    let _ = gather_complete.recv().await;
    
    tokio::signal::ctrl_c().await?;

    Ok(())
}
