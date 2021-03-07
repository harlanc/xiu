use crate::amf0::errors::Amf0ReadError;
use crate::protocol_control_messages::errors::ProtocolControlMessageReaderError;
use netio::netio::bytes_errors::BytesReadError;

pub enum MessageErrorValue {
    BytesReadError(BytesReadError),
    UnknowReadState,
    Amf0ReadError(Amf0ReadError),
    UnknowMessageType,
    ProtocolControlMessageReaderError(ProtocolControlMessageReaderError),
    //IO(io::Error),
}

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
