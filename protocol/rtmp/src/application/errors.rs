use tokio::sync::broadcast::error::{RecvError, SendError};

use {crate::cache::errors::CacheError, failure::Fail, std::fmt};

#[derive(Debug)]
pub struct PushError {
    pub value: PushErrorValue,
}

impl fmt::Display for PushError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PushErrorValue {
    #[fail(display = "receive error\n")]
    ReceiveError(RecvError),

    #[fail(display = "send error\n")]
    SendError,
}

impl From<RecvError> for PushError {
    fn from(error: RecvError) -> Self {
        PushError {
            value: PushErrorValue::ReceiveError(error),
        }
    }
}
