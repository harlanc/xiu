use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use failure::Fail;

#[derive(Debug)]
pub struct RtcpError {
    pub value: RtcpErrorValue,
}

#[derive(Debug, Fail)]
pub enum RtcpErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(BytesWriteError),
}

impl From<BytesReadError> for RtcpError {
    fn from(error: BytesReadError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for RtcpError {
    fn from(error: BytesWriteError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesWriteError(error),
        }
    }
}
