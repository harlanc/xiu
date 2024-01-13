use crate::ps::errors::MpegPsError;

use {
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::{Backtrace, Fail},
    std::fmt,
    std::io::Error,
};

#[derive(Debug, Fail)]
pub enum MpegErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),

    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),

    #[fail(display = "io error")]
    IOError(Error),

    #[fail(display = "program number exists")]
    ProgramNumberExists,

    #[fail(display = "pmt count execeed")]
    PmtCountExeceed,

    #[fail(display = "stream count execeed")]
    StreamCountExeceed,

    #[fail(display = "stream not found")]
    StreamNotFound,

    #[fail(display = "mpeg ps error\n")]
    MpegPsError(MpegPsError),
}
#[derive(Debug)]
pub struct MpegError {
    pub value: MpegErrorValue,
}

impl From<BytesReadError> for MpegError {
    fn from(error: BytesReadError) -> Self {
        MpegError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegError {
    fn from(error: BytesWriteError) -> Self {
        MpegError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Error> for MpegError {
    fn from(error: Error) -> Self {
        MpegError {
            value: MpegErrorValue::IOError(error),
        }
    }
}

impl From<MpegPsError> for MpegError {
    fn from(error: MpegPsError) -> Self {
        MpegError {
            value: MpegErrorValue::MpegPsError(error),
        }
    }
}

impl fmt::Display for MpegError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MpegError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
