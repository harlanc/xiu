use netio::bytes_errors::{BytesReadError,BytesWriteError};
pub enum UnpackErrorValue {
    BytesReadError(BytesReadError),
    UnknowReadState,
    //IO(io::Error),
}

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

pub enum PackErrorValue {
    NotExistHeader,
    UnknowReadState,
    // IO(io::Error),
    BytesWriteError(BytesWriteError),
}

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
