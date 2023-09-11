use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Default)]
pub enum StreamIdentifier {
    #[default]
    Unkonwn,
    Rtmp {
        app_name: String,
        stream_name: String,
    },
    Rtsp {
        stream_path: String,
    },
    WebRTC {
        app_name: String,
        stream_name: String,
    },
}
impl fmt::Display for StreamIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StreamIdentifier::Rtmp {
                app_name,
                stream_name,
            } => {
                write!(f, "RTMP - app_name: {app_name}, stream_name: {stream_name}")
            }
            StreamIdentifier::Rtsp {
                stream_path: stream_name,
            } => {
                write!(f, "RTSP - stream_name: {stream_name}")
            }
            StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            } => {
                write!(
                    f,
                    "WebRTC - app_name: {app_name}, stream_name: {stream_name}"
                )
            }
            StreamIdentifier::Unkonwn => {
                write!(f, "Unkonwn")
            }
        }
    }
}
