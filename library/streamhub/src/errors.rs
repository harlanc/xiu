use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;

use {failure::Fail, std::fmt};
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
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error\n")]
    BytesWriteError(BytesWriteError),
    #[fail(display = "not correct data sender type\n")]
    NotCorrectDataSenderType,
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

impl From<BytesReadError> for ChannelError {
    fn from(error: BytesReadError) -> Self {
        ChannelError {
            value: ChannelErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for ChannelError {
    fn from(error: BytesWriteError) -> Self {
        ChannelError {
            value: ChannelErrorValue::BytesWriteError(error),
        }
    }
}

// impl From<CacheError> for ChannelError {
//     fn from(error: CacheError) -> Self {
//         ChannelError {
//             value: ChannelErrorValue::CacheError(error),
//         }
//     }
// }
