use {
    crate::{
        amf0::errors::Amf0WriteError, cache::errors::MetadataError, session::errors::SessionError,
    },
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::Fail,
    std::fmt,
    tokio::sync::broadcast::error::RecvError,
    xflv::errors::FlvMuxerError,
    xflv::errors::Mpeg4AvcHevcError,
};

pub struct RtmpRemuxerError {
    pub value: RtmpRemuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum RtmpRemuxerErrorValue {
    #[fail(display = "hls error")]
    Error,
    #[fail(display = "session error:{}\n", _0)]
    SessionError(#[cause] SessionError),
    #[fail(display = "amf write error:{}\n", _0)]
    Amf0WriteError(#[cause] Amf0WriteError),
    #[fail(display = "metadata error:{}\n", _0)]
    MetadataError(#[cause] MetadataError),
    #[fail(display = "receive error:{}\n", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "bytes read error:{}\n", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error:{}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "mpeg avc error\n")]
    MpegAvcError(#[cause] Mpeg4AvcHevcError),
    #[fail(display = "flv muxer error\n")]
    FlvMuxerError(#[cause] FlvMuxerError),
    #[fail(display = "stream hub event send error\n")]
    StreamHubEventSendErr,
}
impl From<RecvError> for RtmpRemuxerError {
    fn from(error: RecvError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::RecvError(error),
        }
    }
}

impl From<SessionError> for RtmpRemuxerError {
    fn from(error: SessionError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::SessionError(error),
        }
    }
}

impl From<Amf0WriteError> for RtmpRemuxerError {
    fn from(error: Amf0WriteError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<MetadataError> for RtmpRemuxerError {
    fn from(error: MetadataError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::MetadataError(error),
        }
    }
}

impl From<BytesReadError> for RtmpRemuxerError {
    fn from(error: BytesReadError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for RtmpRemuxerError {
    fn from(error: BytesWriteError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Mpeg4AvcHevcError> for RtmpRemuxerError {
    fn from(error: Mpeg4AvcHevcError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::MpegAvcError(error),
        }
    }
}

impl From<FlvMuxerError> for RtmpRemuxerError {
    fn from(error: FlvMuxerError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::FlvMuxerError(error),
        }
    }
}

impl fmt::Display for RtmpRemuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
