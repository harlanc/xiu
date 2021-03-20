use crate::amf0::errors::Amf0WriteError;

use netio::bytes_errors::BytesWriteError;

use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug)]
pub struct EventMessagesError {
    pub value: EventMessagesErrorValue,
}

#[derive(Debug, Fail)]
pub enum EventMessagesErrorValue {
    #[fail(display = "amf0 write error: {}", _0)]
    Amf0WriteError(Amf0WriteError),
    #[fail(display = "bytes write error: {}", _0)]
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
