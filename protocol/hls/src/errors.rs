use {
    failure::{Backtrace, Fail},
    std::fmt,
    streamhub::errors::StreamHubError,
    tokio::sync::broadcast::error::RecvError,
    tokio::sync::oneshot::error::RecvError as OneshotRecvError,
    xflv::errors::FlvDemuxerError,
    xmpegts::errors::MpegTsError,
};

#[derive(Debug)]
pub struct ServerError {
    pub value: ServerErrorValue,
}

#[derive(Debug, Fail)]
pub enum ServerErrorValue {
    #[fail(display = "server error")]
    Error,
}
#[derive(Debug)]
pub struct MediaError {
    pub value: MediaErrorValue,
}

#[derive(Debug, Fail)]
pub enum MediaErrorValue {
    #[fail(display = "server error")]
    Error,
    #[fail(display = "channel recv error")]
    ChannelRecvError,
    #[fail(display = "flv demuxer error:{}", _0)]
    FlvDemuxerError(#[cause] FlvDemuxerError),
    #[fail(display = "mpegts error:{}", _0)]
    MpegTsError(#[cause] MpegTsError),
    #[fail(display = "write file error:{}", _0)]
    IOError(#[cause] std::io::Error),
}

impl From<FlvDemuxerError> for MediaError {
    fn from(error: FlvDemuxerError) -> Self {
        MediaError {
            value: MediaErrorValue::FlvDemuxerError(error),
        }
    }
}

impl From<MpegTsError> for MediaError {
    fn from(error: MpegTsError) -> Self {
        MediaError {
            value: MediaErrorValue::MpegTsError(error),
        }
    }
}

impl From<std::io::Error> for MediaError {
    fn from(error: std::io::Error) -> Self {
        MediaError {
            value: MediaErrorValue::IOError(error),
        }
    }
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MediaError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

pub struct HlsError {
    pub value: HlsErrorValue,
}

#[derive(Debug, Fail)]
pub enum HlsErrorValue {
    #[fail(display = "hls error")]
    Error,
    #[fail(display = "channel recv error")]
    ChannelRecvError,
    #[fail(display = "channel error:{}", _0)]
    ChannelError(#[cause] StreamHubError),
    #[fail(display = "flv demuxer error:{}", _0)]
    FlvDemuxerError(#[cause] FlvDemuxerError),
    #[fail(display = "media error:{}", _0)]
    MediaError(#[cause] MediaError),
    #[fail(display = "receive error:{}", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "tokio: oneshot receiver err: {}", _0)]
    OneshotRecvError(#[cause] OneshotRecvError),
    #[fail(display = "stream hub event send error")]
    StreamHubEventSendErr,
}
impl From<RecvError> for HlsError {
    fn from(error: RecvError) -> Self {
        HlsError {
            value: HlsErrorValue::RecvError(error),
        }
    }
}

impl From<MediaError> for HlsError {
    fn from(error: MediaError) -> Self {
        HlsError {
            value: HlsErrorValue::MediaError(error),
        }
    }
}

impl From<FlvDemuxerError> for HlsError {
    fn from(error: FlvDemuxerError) -> Self {
        HlsError {
            value: HlsErrorValue::FlvDemuxerError(error),
        }
    }
}

impl From<StreamHubError> for HlsError {
    fn from(error: StreamHubError) -> Self {
        HlsError {
            value: HlsErrorValue::ChannelError(error),
        }
    }
}

impl From<OneshotRecvError> for HlsError {
    fn from(error: OneshotRecvError) -> Self {
        HlsError {
            value: HlsErrorValue::OneshotRecvError(error),
        }
    }
}

impl fmt::Display for HlsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
