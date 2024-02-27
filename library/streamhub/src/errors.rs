use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use failure::Backtrace;

use {failure::Fail, std::fmt};
#[derive(Debug, Fail)]
pub enum StreamHubErrorValue {
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
pub struct StreamHubError {
    pub value: StreamHubErrorValue,
}

impl fmt::Display for StreamHubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for StreamHubError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl From<BytesReadError> for StreamHubError {
    fn from(error: BytesReadError) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for StreamHubError {
    fn from(error: BytesWriteError) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::BytesWriteError(error),
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
