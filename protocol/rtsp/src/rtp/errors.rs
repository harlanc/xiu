use {
    failure::{Backtrace, Fail},
    std::fmt,
};

use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;

#[derive(Debug)]
pub struct PackerError {
    pub value: PackerErrorValue,
}

impl Fail for PackerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for PackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PackerErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for PackerError {
    fn from(error: BytesReadError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for PackerError {
    fn from(error: BytesWriteError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesWriteError(error),
        }
    }
}

#[derive(Debug)]
pub struct UnPackerError {
    pub value: UnPackerErrorValue,
}

#[derive(Debug, Fail)]
pub enum UnPackerErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for UnPackerError {
    fn from(error: BytesReadError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for UnPackerError {
    fn from(error: BytesWriteError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for UnPackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for UnPackerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
