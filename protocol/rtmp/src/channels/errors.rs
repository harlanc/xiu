use crate::cache::errors::{CacheError, CacheErrorValue};

pub enum ChannelErrorValue {
    NoAppName,
    NoStreamName,
    Exists,
    SendError,
    CacheError(CacheError),
}

pub struct ChannelError {
    pub value: ChannelErrorValue,
}

impl From<CacheError> for ChannelError {
    fn from(error: CacheError) -> Self {
        ChannelError {
            value: ChannelErrorValue::CacheError(error),
        }
    }
}
