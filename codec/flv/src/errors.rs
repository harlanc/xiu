use failure::{Backtrace, Fail};
use std::fmt;

use networkio::bytes_errors::BytesReadError;
use networkio::bytes_errors::BytesWriteError;

#[derive(Debug, Fail)]
pub enum TagParseErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),
    #[fail(display = "tag data length error\n")]
    TagDataLength,
    #[fail(display = "unknow tag type error\n")]
    UnknownTagType,
}
#[derive(Debug)]
pub struct TagParseError {
    pub value: TagParseErrorValue,
}

impl From<BytesReadError> for TagParseError {
    fn from(error: BytesReadError) -> Self {
        TagParseError {
            value: TagParseErrorValue::BytesReadError(error),
        }
    }
}

impl fmt::Display for TagParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for TagParseError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

pub struct MuxerError {
    pub value: MuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum MuxerErrorValue {
    // #[fail(display = "server error")]
    // Error,
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
}

impl From<BytesWriteError> for MuxerError {
    fn from(error: BytesWriteError) -> Self {
        MuxerError {
            value: MuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for MuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
