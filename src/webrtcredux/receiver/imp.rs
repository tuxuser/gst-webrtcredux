use std::sync::{Mutex, Arc};
use std::time::Duration;

use bytes::Bytes;
use futures::executor::block_on;
use gst::prelude::ClockExtManual;
use gst::traits::ClockExt;
use gst::{Buffer, FlowError, FlowSuccess, glib, gst_trace as trace, ClockTime};
use gst::subclass::ElementMetadata;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::*;
use once_cell::sync::Lazy;
use tokio::runtime::Handle;
use webrtc::track::track_remote::TrackRemote;

#[derive(PartialEq, Eq)]
pub enum MediaType {
    Video,
    Audio
}

#[derive(Default)]
struct State {
    track: Option<Arc<TrackRemote>>,
    duration: Option<ClockTime>,
    handle: Option<Handle>,
    media_type: Option<MediaType>
}

#[derive(Default)]
pub struct WebRtcReduxReceiver {
    state: Mutex<State>,
}

impl WebRtcReduxReceiver {
    pub fn add_info(&self, track: Arc<TrackRemote>, handle: Handle, media_type: MediaType, duration: Option<ClockTime>) {
        let _ = self.state.lock().unwrap().track.insert(track);
        let _ = self.state.lock().unwrap().handle.insert(handle);
        let _ = self.state.lock().unwrap().media_type.insert(media_type);
        self.state.lock().unwrap().duration = duration;
    }
}

impl ElementImpl for WebRtcReduxReceiver {
    fn metadata() -> Option<&'static ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "WebRTC Broadcast Engine (Internal receiver)",
                "Src/Video/Audio",
                "Internal WebRtcRedux receiver",
                "Jack Hogan; Lorenzo Rizzotti <dev@dreaming.codes>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
            let caps = gst::Caps::builder_full()
                .structure(gst::Structure::builder("audio/x-opus").build())
                .structure(gst::Structure::builder("audio/G722").build())
                .structure(gst::Structure::builder("audio/x-mulaw").build())
                .structure(gst::Structure::builder("audio/x-alaw").build())
                .structure(gst::Structure::builder("video/x-h264").field("stream-format", "byte-stream").field("profile", "baseline").build())
                .structure(gst::Structure::builder("video/x-vp8").build())
                .structure(gst::Structure::builder("video/x-vp9").build())
                .build();
            let sink_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Sometimes,
                &caps,
            )
                .unwrap();

            vec![sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }

    fn change_state(&self, element: &Self::Type, transition: gst::StateChange) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        if transition == gst::StateChange::PausedToPlaying {
            if let Some(duration) = self.state.lock().unwrap().duration {
                self.set_clock(element, Some(&format_clock(duration)));
            }
        }

        self.parent_change_state(element, transition)
    }
}

impl BaseSrcImpl for WebRtcReduxReceiver {
    fn fill(
        &self,
        element: &Self::Type,
        offset: u64,
        length: u32,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        self.parent_fill(element, offset, length, buffer)
    }
}

#[glib::object_subclass]
impl ObjectSubclass for WebRtcReduxReceiver {
    const NAME: &'static str = "WebRtcReduxReceiver";
    type Type = super::WebRtcReduxReceiver;
    type ParentType = gst_base::BaseSrc;
}

impl ObjectImpl for WebRtcReduxReceiver {}

impl GstObjectImpl for WebRtcReduxReceiver {}

fn format_clock(duration: ClockTime) -> gst::Clock {
    let clock = gst::SystemClock::obtain();
    let _ = clock.new_periodic_id(clock.internal_time(), duration);

    clock
}