use bytesio::bits_errors::BitError;
use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug, Fail)]
pub enum H264ErrorValue {
    #[fail(display = "bit error")]
    BitError(BitError),
}
#[derive(Debug)]
pub struct H264Error {
    pub value: H264ErrorValue,
}

impl From<BitError> for H264Error {
    fn from(error: BitError) -> Self {
        H264Error {
            value: H264ErrorValue::BitError(error),
        }
    }
}

impl fmt::Display for H264Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for H264Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
