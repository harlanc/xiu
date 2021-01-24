use crate::amf0::error::{Amf0WriteError, Amf0WriteErrorValue};
use failure::{Backtrace, Fail};
use liverust_lib::netio::writer::IOWriteError;
use std::fmt;
use std::io;
pub struct ControlMessagesError {
    pub value: ControlMessagesErrorValue,
}

pub enum ControlMessagesErrorValue {
    Amf0WriteError(Amf0WriteError),
    IOWriteError(IOWriteError),
}

impl From<Amf0WriteError> for ControlMessagesError {
    fn from(error: Amf0WriteError) -> Self {
        ControlMessagesError {
            value: ControlMessagesErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<IOWriteError> for ControlMessagesError {
    fn from(error: IOWriteError) -> Self {
        ControlMessagesError {
            value: ControlMessagesErrorValue::IOWriteError(error),
        }
    }
}
