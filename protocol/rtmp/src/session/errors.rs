use {
    crate::{
        cache::errors::CacheError,
        chunk::errors::{PackError, UnpackError},
        handshake::errors::HandshakeError,
        messages::errors::MessageError,
        netconnection::errors::NetConnectionError,
        netstream::errors::NetStreamError,
        protocol_control_messages::errors::ControlMessagesError,
        user_control_messages::errors::EventMessagesError,
    },
    bytesio::{bytes_errors::BytesWriteError, bytesio_errors::BytesIOError},
    commonlib::errors::AuthError,
    failure::{Backtrace, Fail},
    std::fmt,
    streamhub::errors::StreamHubError,
    tokio::sync::oneshot::error::RecvError,
    xflv::amf0::errors::Amf0WriteError,
};

#[derive(Debug)]
pub struct SessionError {
    pub value: SessionErrorValue,
}

#[derive(Debug, Fail)]
pub enum SessionErrorValue {
    #[fail(display = "amf0 write error: {}", _0)]
    Amf0WriteError(#[cause] Amf0WriteError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    // #[fail(display = "timeout error: {}", _0)]
    // TimeoutError(#[cause] Elapsed),
    #[fail(display = "unpack error: {}", _0)]
    UnPackError(#[cause] UnpackError),

    #[fail(display = "message error: {}", _0)]
    MessageError(#[cause] MessageError),
    #[fail(display = "control message error: {}", _0)]
    ControlMessagesError(#[cause] ControlMessagesError),
    #[fail(display = "net connection error: {}", _0)]
    NetConnectionError(#[cause] NetConnectionError),
    #[fail(display = "net stream error: {}", _0)]
    NetStreamError(#[cause] NetStreamError),

    #[fail(display = "event messages error: {}", _0)]
    EventMessagesError(#[cause] EventMessagesError),
    #[fail(display = "net io error: {}", _0)]
    BytesIOError(#[cause] BytesIOError),
    #[fail(display = "pack error: {}", _0)]
    PackError(#[cause] PackError),
    #[fail(display = "handshake error: {}", _0)]
    HandshakeError(#[cause] HandshakeError),
    #[fail(display = "cache error name: {}", _0)]
    CacheError(#[cause] CacheError),
    #[fail(display = "tokio: oneshot receiver err: {}", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "streamhub channel err: {}", _0)]
    ChannelError(#[cause] StreamHubError),

    #[fail(display = "amf0 count not correct error")]
    Amf0ValueCountNotCorrect,
    #[fail(display = "amf0 value type not correct error")]
    Amf0ValueTypeNotCorrect,
    #[fail(display = "stream hub event send error")]
    StreamHubEventSendErr,
    #[fail(display = "none frame data sender error")]
    NoneFrameDataSender,
    #[fail(display = "none frame data receiver error")]
    NoneFrameDataReceiver,
    #[fail(display = "send frame data error")]
    SendFrameDataErr,
    #[fail(display = "subscribe count limit is reached.")]
    SubscribeCountLimitReach,

    #[fail(display = "no app name error")]
    NoAppName,
    #[fail(display = "no media data can be received now.")]
    NoMediaDataReceived,

    #[fail(display = "session is finished.")]
    Finish,
    #[fail(display = "Auth err: {}", _0)]
    AuthError(#[cause] AuthError),
}

impl From<Amf0WriteError> for SessionError {
    fn from(error: Amf0WriteError) -> Self {
        SessionError {
            value: SessionErrorValue::Amf0WriteError(error),
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

// impl From<Elapsed> for SessionError {
//     fn from(error: Elapsed) -> Self {
//         SessionError {
//             value: SessionErrorValue::TimeoutError(error),
//         }
//     }
// }

impl From<UnpackError> for SessionError {
    fn from(error: UnpackError) -> Self {
        SessionError {
            value: SessionErrorValue::UnPackError(error),
        }
    }
}

impl From<MessageError> for SessionError {
    fn from(error: MessageError) -> Self {
        SessionError {
            value: SessionErrorValue::MessageError(error),
        }
    }
}

impl From<ControlMessagesError> for SessionError {
    fn from(error: ControlMessagesError) -> Self {
        SessionError {
            value: SessionErrorValue::ControlMessagesError(error),
        }
    }
}

impl From<NetConnectionError> for SessionError {
    fn from(error: NetConnectionError) -> Self {
        SessionError {
            value: SessionErrorValue::NetConnectionError(error),
        }
    }
}

impl From<NetStreamError> for SessionError {
    fn from(error: NetStreamError) -> Self {
        SessionError {
            value: SessionErrorValue::NetStreamError(error),
        }
    }
}

impl From<EventMessagesError> for SessionError {
    fn from(error: EventMessagesError) -> Self {
        SessionError {
            value: SessionErrorValue::EventMessagesError(error),
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

impl From<PackError> for SessionError {
    fn from(error: PackError) -> Self {
        SessionError {
            value: SessionErrorValue::PackError(error),
        }
    }
}

impl From<HandshakeError> for SessionError {
    fn from(error: HandshakeError) -> Self {
        SessionError {
            value: SessionErrorValue::HandshakeError(error),
        }
    }
}

impl From<CacheError> for SessionError {
    fn from(error: CacheError) -> Self {
        SessionError {
            value: SessionErrorValue::CacheError(error),
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

impl From<StreamHubError> for SessionError {
    fn from(error: StreamHubError) -> Self {
        SessionError {
            value: SessionErrorValue::ChannelError(error),
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
