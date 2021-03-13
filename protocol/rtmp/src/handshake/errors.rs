use netio::bytes_errors::{BytesReadError, BytesWriteError};

use std::time::SystemTimeError;

pub enum HandshakeErrorValue {
    BytesReadError(BytesReadError),
    BytesWriteError(BytesWriteError),
    SysTimeError(SystemTimeError),
    DigestNotFound,
    S0VersionNotCorrect,
}

pub struct HandshakeError {
    pub value: HandshakeErrorValue,
}

impl From<HandshakeErrorValue> for HandshakeError {
    fn from(val: HandshakeErrorValue) -> Self {
        HandshakeError { value: val }
    }
}

impl From<BytesReadError> for HandshakeError {
    fn from(error: BytesReadError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for HandshakeError {
    fn from(error: BytesWriteError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::BytesWriteError(error),
        }
    }
}

impl From<SystemTimeError> for HandshakeError {
    fn from(error: SystemTimeError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::SysTimeError(error),
        }
    }
}
