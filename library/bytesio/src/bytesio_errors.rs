use failure::{Backtrace, Fail};
use std::fmt;
use std::io;
// use tokio::time::Elapsed;

#[derive(Debug, Fail)]
pub enum BytesIOErrorValue {
    #[fail(display = "not enough bytes")]
    NotEnoughBytes,
    #[fail(display = "empty stream")]
    EmptyStream,
    #[fail(display = "io error")]
    IOError(io::Error),
    #[fail(display = "time out error")]
    TimeoutError(tokio::time::error::Elapsed),
    #[fail(display = "none return")]
    NoneReturn,
}
#[derive(Debug)]
pub struct BytesIOError {
    pub value: BytesIOErrorValue,
}

impl From<BytesIOErrorValue> for BytesIOError {
    fn from(val: BytesIOErrorValue) -> Self {
        BytesIOError { value: val }
    }
}

impl From<io::Error> for BytesIOError {
    fn from(error: io::Error) -> Self {
        BytesIOError {
            value: BytesIOErrorValue::IOError(error),
        }
    }
}

// impl From<Elapsed> for NetIOError {
//     fn from(error: Elapsed) -> Self {
//         NetIOError {
//             value: NetIOErrorValue::TimeoutError(error),
//         }
//     }
// }

impl fmt::Display for BytesIOError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for BytesIOError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
