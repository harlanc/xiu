use crate::amf0::error::{Amf0WriteError, Amf0WriteErrorValue};
use failure::{Backtrace, Fail};
use std::fmt;
use std::io;

pub struct NetConnectionError {
    pub value: NetConnectionErrorValue,
}

pub enum NetConnectionErrorValue {
    Amf0WriteError(Amf0WriteError),
}

impl From<Amf0WriteError> for NetConnectionError {
    fn from(error: Amf0WriteError) -> Self {
        NetConnectionError {
            value: NetConnectionErrorValue::Amf0WriteError(error),
        }
    }
}
