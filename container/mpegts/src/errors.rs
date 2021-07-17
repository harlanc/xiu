use failure::{Backtrace, Fail};
use std::fmt;

use networkio::bytes_errors::BytesReadError;
use networkio::bytes_errors::BytesWriteError;

#[derive(Debug, Fail)]
pub enum MpegTsErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),

    #[fail(display = "bytes write error\n")]
    BytesWriteError(BytesWriteError),
}
#[derive(Debug)]
pub struct MpegTsError {
    pub value: MpegTsErrorValue,
}

impl From<BytesReadError> for MpegTsError {
    fn from(error: BytesReadError) -> Self {
        MpegTsError {
            value: MpegTsErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegTsError {
    fn from(error: BytesWriteError) -> Self {
        MpegTsError {
            value: MpegTsErrorValue::BytesWriteError(error),
        }
    }
}
