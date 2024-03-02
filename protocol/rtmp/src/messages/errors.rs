use {
    crate::{
        protocol_control_messages::errors::ProtocolControlMessageReaderError,
        user_control_messages::errors::EventMessagesError,
    },
    bytesio::bytes_errors::BytesReadError,
    failure::{Backtrace, Fail},
    std::fmt,
    xflv::amf0::errors::Amf0ReadError,
};

#[derive(Debug, Fail)]
pub enum MessageErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "unknow read state")]
    UnknowReadState,
    #[fail(display = "amf0 read error: {}", _0)]
    Amf0ReadError(Amf0ReadError),
    #[fail(display = "unknown message type")]
    UnknowMessageType,
    #[fail(display = "protocol control message read error: {}", _0)]
    ProtocolControlMessageReaderError(ProtocolControlMessageReaderError),
    #[fail(display = "user control message read error: {}", _0)]
    EventMessagesError(EventMessagesError),
}

#[derive(Debug)]
pub struct MessageError {
    pub value: MessageErrorValue,
}

impl From<MessageErrorValue> for MessageError {
    fn from(val: MessageErrorValue) -> Self {
        MessageError { value: val }
    }
}

impl From<BytesReadError> for MessageError {
    fn from(error: BytesReadError) -> Self {
        MessageError {
            value: MessageErrorValue::BytesReadError(error),
        }
    }
}

impl From<Amf0ReadError> for MessageError {
    fn from(error: Amf0ReadError) -> Self {
        MessageError {
            value: MessageErrorValue::Amf0ReadError(error),
        }
    }
}

impl From<ProtocolControlMessageReaderError> for MessageError {
    fn from(error: ProtocolControlMessageReaderError) -> Self {
        MessageError {
            value: MessageErrorValue::ProtocolControlMessageReaderError(error),
        }
    }
}

impl From<EventMessagesError> for MessageError {
    fn from(error: EventMessagesError) -> Self {
        MessageError {
            value: MessageErrorValue::EventMessagesError(error),
        }
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MessageError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
