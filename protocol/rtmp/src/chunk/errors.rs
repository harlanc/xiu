use liverust_lib::netio::bytes_errors::BytesReadError;
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