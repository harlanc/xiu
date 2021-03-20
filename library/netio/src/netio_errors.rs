use failure::{Backtrace, Fail};
use std::fmt;
use std::io;
use tokio::time::Elapsed;

#[derive(Debug, Fail)]
pub enum NetIOErrorValue {
    #[fail(display = "not enough bytes")]
    NotEnoughBytes,
    #[fail(display = "empty stream")]
    EmptyStream,
    #[fail(display = "io error")]
    IOError(io::Error),
    #[fail(display = "time out error")]
    TimeoutError(Elapsed),
    #[fail(display = "none return")]
    NoneReturn,
}
#[derive(Debug)]
pub struct NetIOError {
    pub value: NetIOErrorValue,
}

impl From<NetIOErrorValue> for NetIOError {
    fn from(val: NetIOErrorValue) -> Self {
        NetIOError { value: val }
    }
}

impl From<io::Error> for NetIOError {
    fn from(error: io::Error) -> Self {
        NetIOError {
            value: NetIOErrorValue::IOError(error),
        }
    }
}

impl From<Elapsed> for NetIOError {
    fn from(error: Elapsed) -> Self {
        NetIOError {
            value: NetIOErrorValue::TimeoutError(error),
        }
    }
}

impl fmt::Display for NetIOError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for NetIOError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
