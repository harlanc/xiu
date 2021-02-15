use crate::amf0::errors::{Amf0WriteError, Amf0WriteErrorValue};
use failure::{Backtrace, Fail};
use liverust_lib::netio::errors::IOReadError;
use liverust_lib::netio::errors::IOWriteError;
use std::fmt;
use std::io;
pub struct ControlMessagesError {
    pub value: ControlMessagesErrorValue,
}

pub enum ControlMessagesErrorValue {
    //Amf0WriteError(Amf0WriteError),
    IOWriteError(IOWriteError),
}

// impl From<Amf0WriteError> for ControlMessagesError {
//     fn from(error: Amf0WriteError) -> Self {
//         ControlMessagesError {
//             value: ControlMessagesErrorValue::Amf0WriteError(error),
//         }
//     }
// }

impl From<IOWriteError> for ControlMessagesError {
    fn from(error: IOWriteError) -> Self {
        ControlMessagesError {
            value: ControlMessagesErrorValue::IOWriteError(error),
        }
    }
}

pub struct ProtocolControlMessageReaderError {
    pub value: ProtocolControlMessageReaderErrorValue,
}

pub enum ProtocolControlMessageReaderErrorValue {
    IOReadError(IOReadError),
}

impl From<IOReadError> for ProtocolControlMessageReaderError {
    fn from(error: IOReadError) -> Self {
        ProtocolControlMessageReaderError {
            value: ProtocolControlMessageReaderErrorValue::IOReadError(error),
        }
    }
}
