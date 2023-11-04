use {
    failure::Fail,
    std::{fmt, io::Error},
    tokio::sync::broadcast::error::RecvError,
};

#[derive(Debug)]
pub struct ClientError {
    pub value: PushClientErrorValue,
}

impl fmt::Display for ClientError {
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

impl From<Error> for ClientError {
    fn from(error: Error) -> Self {
        ClientError {
            value: PushClientErrorValue::IOError(error),
        }
    }
}

impl From<RecvError> for ClientError {
    fn from(error: RecvError) -> Self {
        ClientError {
            value: PushClientErrorValue::ReceiveError(error),
        }
    }
}
