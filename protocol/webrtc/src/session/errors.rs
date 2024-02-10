use streamhub::errors::ChannelError;
use {
    bytesio::bytes_errors::BytesReadError,
    bytesio::{bytes_errors::BytesWriteError, bytesio_errors::BytesIOError},
    commonlib::errors::AuthError,
    failure::{Backtrace, Fail},
    std::fmt,
    std::str::Utf8Error,
    tokio::sync::oneshot::error::RecvError,
    webrtc::error::Error as RTCError,
};

#[derive(Debug)]
pub struct SessionError {
    pub value: SessionErrorValue,
}

#[derive(Debug, Fail)]
pub enum SessionErrorValue {
    #[fail(display = "net io error: {}", _0)]
    BytesIOError(#[cause] BytesIOError),
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "Utf8Error: {}", _0)]
    Utf8Error(#[cause] Utf8Error),
    #[fail(display = "event execute error: {}", _0)]
    ChannelError(#[cause] ChannelError),
    #[fail(display = "webrtc error: {}", _0)]
    RTCError(#[cause] RTCError),
    #[fail(display = "tokio: oneshot receiver err: {}", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "Auth err: {}", _0)]
    AuthError(#[cause] AuthError),
    #[fail(display = "stream hub event send error")]
    StreamHubEventSendErr,
    #[fail(display = "cannot receive frame data from stream hub")]
    CannotReceiveFrameData,
    #[fail(display = "Http Request path error")]
    HttpRequestPathError,
    #[fail(display = "Not supported")]
    HttpRequestNotSupported,
    #[fail(display = "Empty sdp data")]
    HttpRequestEmptySdp,
    #[fail(display = "Cannot find Content-Length")]
    HttpRequestNoContentLength,
    #[fail(display = "Channel receive error")]
    ChannelRecvError,
}

impl From<RTCError> for SessionError {
    fn from(error: RTCError) -> Self {
        SessionError {
            value: SessionErrorValue::RTCError(error),
        }
    }
}

impl From<BytesIOError> for SessionError {
    fn from(error: BytesIOError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesIOError(error),
        }
    }
}

impl From<BytesReadError> for SessionError {
    fn from(error: BytesReadError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for SessionError {
    fn from(error: BytesWriteError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Utf8Error> for SessionError {
    fn from(error: Utf8Error) -> Self {
        SessionError {
            value: SessionErrorValue::Utf8Error(error),
        }
    }
}

impl From<ChannelError> for SessionError {
    fn from(error: ChannelError) -> Self {
        SessionError {
            value: SessionErrorValue::ChannelError(error),
        }
    }
}

impl From<RecvError> for SessionError {
    fn from(error: RecvError) -> Self {
        SessionError {
            value: SessionErrorValue::RecvError(error),
        }
    }
}

impl From<AuthError> for SessionError {
    fn from(error: AuthError) -> Self {
        SessionError {
            value: SessionErrorValue::AuthError(error),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for SessionError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
