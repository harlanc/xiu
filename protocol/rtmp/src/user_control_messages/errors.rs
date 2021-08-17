use {
    crate::amf0::errors::Amf0WriteError,
    failure::{Backtrace, Fail},
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    std::fmt,
};

#[derive(Debug)]
pub struct EventMessagesError {
    pub value: EventMessagesErrorValue,
}

#[derive(Debug, Fail)]
pub enum EventMessagesErrorValue {
    #[fail(display = "amf0 write error: {}\n", _0)]
    Amf0WriteError(Amf0WriteError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(BytesWriteError),
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "unknow event message type")]
    UnknowEventMessageType,
}

impl From<Amf0WriteError> for EventMessagesError {
    fn from(error: Amf0WriteError) -> Self {
        EventMessagesError {
            value: EventMessagesErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<BytesWriteError> for EventMessagesError {
    fn from(error: BytesWriteError) -> Self {
        EventMessagesError {
            value: EventMessagesErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BytesReadError> for EventMessagesError {
    fn from(error: BytesReadError) -> Self {
        EventMessagesError {
            value: EventMessagesErrorValue::BytesReadError(error),
        }
    }
}

impl fmt::Display for EventMessagesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for EventMessagesError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
