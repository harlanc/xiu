

use liverust_lib::netio::errors::IOReadError;
pub enum UnpackErrorValue {
    IO(IOReadError),
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

impl From<IOReadError> for UnpackError {
    fn from(error: IOReadError) -> Self {
        UnpackError {
            value: UnpackErrorValue::IO(error),
        }
    }
}