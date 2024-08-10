use {
    failure::Fail,
    std::{fmt, io::Error},
    tokio::sync::broadcast::error::RecvError,
};

#[derive(Debug)]
pub struct RelayError {
    pub value: PushClientErrorValue,
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PushClientErrorValue {
    #[fail(display = "receive error")]
    ReceiveError(RecvError),

    #[fail(display = "send error")]
    SendError,
    #[fail(display = "io error")]
    IOError(Error),
}

impl From<Error> for RelayError {
    fn from(error: Error) -> Self {
        RelayError {
            value: PushClientErrorValue::IOError(error),
        }
    }
}

impl From<RecvError> for RelayError {
    fn from(error: RecvError) -> Self {
        RelayError {
            value: PushClientErrorValue::ReceiveError(error),
        }
    }
}
