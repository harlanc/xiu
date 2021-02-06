use crate::{amf0::errors::{Amf0WriteError, Amf0WriteErrorValue}};
use failure::{Backtrace, Fail};
use liverust_lib::netio::writer::IOWriteError;
use std::fmt;
use std::io;
use crate::chunk::errors::UnpackError;
use {
   
    //thiserror::Error,
    tokio::time::Elapsed
    //crate::proto::Error as ProtocolError,
};

pub struct ServerError {
    pub value: ServerErrorValue,
}

pub enum ServerErrorValue {
    Amf0WriteError(Amf0WriteError),
    IOWriteError(IOWriteError),
    //IOError(Error),
    TimeoutError(Elapsed),
    UnPackError(UnpackError),

}

// impl From<Error> for ServerError {
//     fn from(error: Error) -> Self {
//         ServerError {
//             value: ServerErrorValue::IOError(error),
//         }
//     }
// }

impl From<Amf0WriteError> for ServerError {
    fn from(error: Amf0WriteError) -> Self {
        ServerError {
            value: ServerErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<IOWriteError> for ServerError {
    fn from(error: IOWriteError) -> Self {
        ServerError {
            value: ServerErrorValue::IOWriteError(error),
        }
    }
}

impl From<Elapsed> for ServerError {
    fn from(error: Elapsed) -> Self {
        ServerError {
            value: ServerErrorValue::TimeoutError(error),
        }
    }
}

impl From<UnpackError> for ServerError {
    fn from(error: UnpackError) -> Self {
        ServerError {
            value: ServerErrorValue::UnPackError(error),
        }
    }
}
