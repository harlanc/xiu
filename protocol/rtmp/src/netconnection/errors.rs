use crate::amf0::errors::Amf0ReadError;
use crate::amf0::errors::Amf0WriteError;

pub struct NetConnectionError {
    pub value: NetConnectionErrorValue,
}

pub enum NetConnectionErrorValue {
    Amf0WriteError(Amf0WriteError),
    Amf0ReadError(Amf0ReadError),
}

impl From<Amf0WriteError> for NetConnectionError {
    fn from(error: Amf0WriteError) -> Self {
        NetConnectionError {
            value: NetConnectionErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<Amf0ReadError> for NetConnectionError {
    fn from(error: Amf0ReadError) -> Self {
        NetConnectionError {
            value: NetConnectionErrorValue::Amf0ReadError(error),
        }
    }
}
