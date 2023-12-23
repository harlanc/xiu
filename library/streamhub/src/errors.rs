use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use failure::Backtrace;

use {failure::Fail, std::fmt};
#[derive(Debug, Fail)]
pub enum ChannelErrorValue {
    #[fail(display = "no app name")]
    NoAppName,
    #[fail(display = "no stream name")]
    NoStreamName,
    #[fail(display = "no app or stream name")]
    NoAppOrStreamName,
    #[fail(display = "exists")]
    Exists,
    #[fail(display = "send error")]
    SendError,
    #[fail(display = "send video error")]
    SendVideoError,
    #[fail(display = "send audio error")]
    SendAudioError,
    #[fail(display = "bytes read error")]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
    #[fail(display = "not correct data sender type")]
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

impl Fail for ChannelError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
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
