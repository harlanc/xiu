use {crate::cache::errors::CacheError, failure::Fail, std::fmt};
#[derive(Debug, Fail)]
pub enum ChannelErrorValue {
    #[fail(display = "no app name\n")]
    NoAppName,
    #[fail(display = "no stream name\n")]
    NoStreamName,
    #[fail(display = "no app or stream name\n")]
    NoAppOrStreamName,
    #[fail(display = "exists\n")]
    Exists,
    #[fail(display = "send error\n")]
    SendError,
    #[fail(display = "send video error\n")]
    SendVideoError,
    #[fail(display = "send audio error\n")]
    SendAudioError,
    #[fail(display = "cache error name: {}\n", _0)]
    CacheError(CacheError),
}
#[derive(Debug)]
pub struct ChannelError {
    pub value: ChannelErrorValue,
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl From<CacheError> for ChannelError {
    fn from(error: CacheError) -> Self {
        ChannelError {
            value: ChannelErrorValue::CacheError(error),
        }
    }
}
