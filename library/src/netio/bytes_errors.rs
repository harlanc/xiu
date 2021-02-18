use std::io;
use tokio::time::Elapsed;
use super::netio_errors::{NetIOError,NetIOErrorValue};

pub enum IOReadErrorValue {
    NotEnoughBytes,
    EmptyStream,
    IO(io::Error),
    TimeoutError(Elapsed),
}
pub struct IOReadError {
    pub value: IOReadErrorValue,
}

impl From<IOReadErrorValue> for IOReadError {
    fn from(val: IOReadErrorValue) -> Self {
        IOReadError { value: val }
    }
}

impl From<io::Error> for IOReadError {
    fn from(error: io::Error) -> Self {
        IOReadError {
            value: IOReadErrorValue::IO(error),
        }
    }
}

impl From<Elapsed> for IOReadError {
    fn from(error: Elapsed) -> Self {
        IOReadError {
            value: IOReadErrorValue::TimeoutError(error),
        }
    }
}

pub struct BytesWriteError {
    pub value: BytesWriteErrorValue,
}

pub enum BytesWriteErrorValue {
    IO(io::Error),
    NetIOError(NetIOError),
}

impl From<io::Error> for BytesWriteError {
    fn from(error: io::Error) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::IO(error),
        }
    }
}

impl From<NetIOError> for BytesWriteError {
    fn from(error: NetIOError) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::NetIOError(error),
        }
    }
}

