use tokio::sync::broadcast::error::{RecvError, SendError};

use std::io::Error;
use {crate::cache::errors::CacheError, failure::Fail, std::fmt};
#[derive(Debug)]
pub struct PushClientError {
    pub value: PushClientErrorValue,
}

impl fmt::Display for PushClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PushClientErrorValue {
    #[fail(display = "receive error\n")]
    ReceiveError(RecvError),

    #[fail(display = "send error\n")]
    SendError,
    #[fail(display = "io error\n")]
    IOError(Error),
}

impl From<Error> for PushClientError {
    fn from(error: Error) -> Self {
        PushClientError {
            value: PushClientErrorValue::IOError(error),
        }
    }
}

impl From<RecvError> for PushClientError {
    fn from(error: RecvError) -> Self {
        PushClientError {
            value: PushClientErrorValue::ReceiveError(error),
        }
    }
}
