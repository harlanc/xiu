use {
    failure::{Backtrace, Fail},
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    std::fmt,
};

#[derive(Debug, Fail)]
pub enum UnpackErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "unknow read state")]
    UnknowReadState,
    #[fail(display = "empty chunks")]
    EmptyChunks,
    //IO(io::Error),
    #[fail(display = "cannot parse")]
    CannotParse,
}

#[derive(Debug)]
pub struct UnpackError {
    pub value: UnpackErrorValue,
}

impl From<UnpackErrorValue> for UnpackError {
    fn from(val: UnpackErrorValue) -> Self {
        UnpackError { value: val }
    }
}

impl From<BytesReadError> for UnpackError {
    fn from(error: BytesReadError) -> Self {
        UnpackError {
            value: UnpackErrorValue::BytesReadError(error),
        }
    }
}

#[derive(Debug, Fail)]
pub enum PackErrorValue {
    #[fail(display = "not exist header")]
    NotExistHeader,
    #[fail(display = "unknow read state")]
    UnknowReadState,
    #[fail(display = "bytes writer error: {}", _0)]
    BytesWriteError(BytesWriteError),
}

#[derive(Debug)]
pub struct PackError {
    pub value: PackErrorValue,
}

impl From<PackErrorValue> for PackError {
    fn from(val: PackErrorValue) -> Self {
        PackError { value: val }
    }
}

impl From<BytesWriteError> for PackError {
    fn from(error: BytesWriteError) -> Self {
        PackError {
            value: PackErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for PackError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for UnpackError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
