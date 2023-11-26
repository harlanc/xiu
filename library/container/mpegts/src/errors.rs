use {
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::{Backtrace, Fail},
    std::fmt,
    std::io::Error,
};

#[derive(Debug, Fail)]
pub enum MpegTsErrorValue {
    #[fail(display = "bytes read error")]
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
}
#[derive(Debug)]
pub struct MpegTsError {
    pub value: MpegTsErrorValue,
}

impl From<BytesReadError> for MpegTsError {
    fn from(error: BytesReadError) -> Self {
        MpegTsError {
            value: MpegTsErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegTsError {
    fn from(error: BytesWriteError) -> Self {
        MpegTsError {
            value: MpegTsErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Error> for MpegTsError {
    fn from(error: Error) -> Self {
        MpegTsError {
            value: MpegTsErrorValue::IOError(error),
        }
    }
}

impl fmt::Display for MpegTsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MpegTsError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
