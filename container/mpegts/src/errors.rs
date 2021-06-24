use failure::{Backtrace, Fail};
use std::fmt;

use networkio::bytes_errors::BytesReadError;
use networkio::bytes_errors::BytesWriteError;

#[derive(Debug, Fail)]
pub enum MpegTsParseErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),

    #[fail(display = "bytes write error\n")]
    BytesWriteError(BytesWriteError),
}
#[derive(Debug)]
pub struct MpegTsParseError {
    pub value: MpegTsParseErrorValue,
}

impl From<BytesReadError> for MpegTsParseError {
    fn from(error: BytesReadError) -> Self {
        MpegTsParseError {
            value: MpegTsParseErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegTsParseError {
    fn from(error: BytesWriteError) -> Self {
        MpegTsParseError {
            value: MpegTsParseErrorValue::BytesWriteError(error),
        }
    }
}
