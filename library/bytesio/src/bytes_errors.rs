use super::bytesio_errors::BytesIOError;
use std::io;
// use tokio::time::Elapsed;

use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug, Fail)]
pub enum BytesReadErrorValue {
    #[fail(display = "not enough bytes to read")]
    NotEnoughBytes,
    #[fail(display = "empty stream")]
    EmptyStream,
    #[fail(display = "io error: {}", _0)]
    IO(#[cause] io::Error),
    #[fail(display = "index out of range")]
    IndexOutofRange,
    #[fail(display = "bytesio read error: {}", _0)]
    BytesIOError(BytesIOError),
    // #[fail(display = "elapsed: {}", _0)]
    // TimeoutError(#[cause] Elapsed),
}

#[derive(Debug)]
pub struct BytesReadError {
    pub value: BytesReadErrorValue,
}

impl From<BytesReadErrorValue> for BytesReadError {
    fn from(val: BytesReadErrorValue) -> Self {
        BytesReadError { value: val }
    }
}

impl From<io::Error> for BytesReadError {
    fn from(error: io::Error) -> Self {
        BytesReadError {
            value: BytesReadErrorValue::IO(error),
        }
    }
}

impl From<BytesIOError> for BytesReadError {
    fn from(error: BytesIOError) -> Self {
        BytesReadError {
            value: BytesReadErrorValue::BytesIOError(error),
        }
    }
}

// impl From<Elapsed> for BytesReadError {
//     fn from(error: Elapsed) -> Self {
//         BytesReadError {
//             value: BytesReadErrorValue::TimeoutError(error),
//         }
//     }
// }

#[derive(Debug)]
pub struct BytesWriteError {
    pub value: BytesWriteErrorValue,
}

#[derive(Debug, Fail)]
pub enum BytesWriteErrorValue {
    #[fail(display = "io error")]
    IO(io::Error),
    #[fail(display = "bytes io error: {}", _0)]
    BytesIOError(BytesIOError),
    #[fail(display = "write time out")]
    Timeout,
    #[fail(display = "outof index")]
    OutofIndex,
}

impl From<io::Error> for BytesWriteError {
    fn from(error: io::Error) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::IO(error),
        }
    }
}

impl From<BytesIOError> for BytesWriteError {
    fn from(error: BytesIOError) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::BytesIOError(error),
        }
    }
}

impl fmt::Display for BytesReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for BytesReadError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for BytesWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for BytesWriteError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
