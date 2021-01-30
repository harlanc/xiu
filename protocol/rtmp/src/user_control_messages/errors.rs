use crate::amf0::error::{Amf0WriteError, Amf0WriteErrorValue};
use failure::{Backtrace, Fail};
use liverust_lib::netio::writer::IOWriteError;
use std::fmt;
use std::io;
pub struct EventMessagesError {
    pub value: EventMessagesErrorValue,
}

pub enum EventMessagesErrorValue {
    Amf0WriteError(Amf0WriteError),
    IOWriteError(IOWriteError),
}

impl From<Amf0WriteError> for EventMessagesError {
    fn from(error: Amf0WriteError) -> Self {
        EventMessagesError {
            value: EventMessagesErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<IOWriteError> for EventMessagesError {
    fn from(error: IOWriteError) -> Self {
        EventMessagesError {
            value: EventMessagesErrorValue::IOWriteError(error),
        }
    }
}
