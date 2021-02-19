use crate::amf0::errors::Amf0WriteError;

use liverust_lib::netio::bytes_errors::BytesWriteError;

pub struct EventMessagesError {
    pub value: EventMessagesErrorValue,
}

pub enum EventMessagesErrorValue {
    Amf0WriteError(Amf0WriteError),
    BytesWriteError(BytesWriteError),
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
