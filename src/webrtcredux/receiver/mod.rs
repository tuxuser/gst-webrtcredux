use std::sync::Arc;

use gst::{glib, ClockTime};
use gst::subclass::prelude::ObjectSubclassExt;

mod imp;

pub use imp::*;
use tokio::runtime::Handle;
use webrtc::track::track_remote::TrackRemote;

glib::wrapper! {
    pub struct WebRtcReduxReceiver(ObjectSubclass<imp::WebRtcReduxReceiver>) @extends gst_base::BaseSink, gst::Element, gst::Object;
}

impl WebRtcReduxReceiver {
    pub fn add_info(&self, track: Arc<TrackRemote>, handle: Handle, media_type: MediaType, duration: Option<ClockTime>) {
        imp::WebRtcReduxReceiver::from_instance(self).add_info(track, handle, media_type, duration);
    }
}

unsafe impl Send for WebRtcReduxReceiver {}

impl Default for WebRtcReduxReceiver {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}