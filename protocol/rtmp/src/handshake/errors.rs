use {
    failure::{Backtrace, Fail},
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    std::{fmt, io::Error, time::SystemTimeError},
};

#[derive(Debug, Fail)]
pub enum HandshakeErrorValue {
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(BytesWriteError),
    #[fail(display = "system time error: {}\n", _0)]
    SysTimeError(SystemTimeError),
    #[fail(display = "Digest not found error\n")]
    DigestNotFound,
    #[fail(display = "s0 version not correct error\n")]
    S0VersionNotCorrect,
    #[fail(display = "io error\n")]
    IOError(Error),
}

impl From<Error> for HandshakeError {
    fn from(error: Error) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IOError(error),
        }
    }
}

#[derive(Debug)]
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

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for HandshakeError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
