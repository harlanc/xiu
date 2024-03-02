use {
    crate::{cache::errors::MetadataError, session::errors::SessionError},
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::Fail,
    std::fmt,
    streamhub::errors::StreamHubError,
    tokio::sync::broadcast::error::RecvError,
    tokio::sync::oneshot::error::RecvError as OneshotRecvError,
    xflv::amf0::errors::Amf0WriteError,
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
    #[fail(display = "session error:{}", _0)]
    SessionError(#[cause] SessionError),
    #[fail(display = "amf write error:{}", _0)]
    Amf0WriteError(#[cause] Amf0WriteError),
    #[fail(display = "metadata error:{}", _0)]
    MetadataError(#[cause] MetadataError),
    #[fail(display = "receive error:{}", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "bytes read error:{}", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error:{}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "mpeg avc error")]
    MpegAvcError(#[cause] Mpeg4AvcHevcError),
    #[fail(display = "flv muxer error")]
    FlvMuxerError(#[cause] FlvMuxerError),
    #[fail(display = "stream hub event send error")]
    StreamHubEventSendErr,
    #[fail(display = "event execute error: {}", _0)]
    ChannelError(#[cause] StreamHubError),
    #[fail(display = "tokio: oneshot receiver err: {}", _0)]
    OneshotRecvError(#[cause] OneshotRecvError),
    #[fail(display = "Channel receive error")]
    ChannelRecvError,
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

impl From<StreamHubError> for RtmpRemuxerError {
    fn from(error: StreamHubError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::ChannelError(error),
        }
    }
}

impl From<OneshotRecvError> for RtmpRemuxerError {
    fn from(error: OneshotRecvError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::OneshotRecvError(error),
        }
    }
}

impl fmt::Display for RtmpRemuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
