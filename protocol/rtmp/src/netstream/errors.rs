use crate::amf0::errors::Amf0WriteError;

use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug)]
pub struct NetStreamError {
    pub value: NetStreamErrorValue,
}

#[derive(Debug, Fail)]
pub enum NetStreamErrorValue {
    #[fail(display = "amf0 write error: {}", _0)]
    Amf0WriteError(Amf0WriteError),
    #[fail(display = "invalid max chunk size")]
    InvalidMaxChunkSize { chunk_size: usize },
}

impl From<Amf0WriteError> for NetStreamError {
    fn from(error: Amf0WriteError) -> Self {
        NetStreamError {
            value: NetStreamErrorValue::Amf0WriteError(error),
        }
    }
}

impl fmt::Display for NetStreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for NetStreamError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
