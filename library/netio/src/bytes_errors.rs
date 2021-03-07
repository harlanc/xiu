use std::io;
use tokio::time::Elapsed;
use super::netio_errors::{NetIOError,NetIOErrorValue};

pub enum BytesReadErrorValue {
    NotEnoughBytes,
    EmptyStream,
    IO(io::Error),
    TimeoutError(Elapsed),
}
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

impl From<Elapsed> for BytesReadError {
    fn from(error: Elapsed) -> Self {
        BytesReadError {
            value: BytesReadErrorValue::TimeoutError(error),
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

