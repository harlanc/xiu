use {
    bytesio::bits_errors::BitError,
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::{Backtrace, Fail},
    std::fmt,
    std::io::Error,
};

#[derive(Debug, Fail)]
pub enum MpegPsErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),

    #[fail(display = "bytes write error\n")]
    BytesWriteError(BytesWriteError),

    #[fail(display = "bits error\n")]
    BitError(BitError),

    #[fail(display = "io error\n")]
    IOError(Error),

    #[fail(display = "start code not correct.\n")]
    StartCodeNotCorrect,
}
#[derive(Debug)]
pub struct MpegPsError {
    pub value: MpegPsErrorValue,
}

impl From<BytesReadError> for MpegPsError {
    fn from(error: BytesReadError) -> Self {
        MpegPsError {
            value: MpegPsErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegPsError {
    fn from(error: BytesWriteError) -> Self {
        MpegPsError {
            value: MpegPsErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BitError> for MpegPsError {
    fn from(error: BitError) -> Self {
        MpegPsError {
            value: MpegPsErrorValue::BitError(error),
        }
    }
}

impl From<Error> for MpegPsError {
    fn from(error: Error) -> Self {
        MpegPsError {
            value: MpegPsErrorValue::IOError(error),
        }
    }
}

impl fmt::Display for MpegPsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MpegPsError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
