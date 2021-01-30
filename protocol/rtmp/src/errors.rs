use crate::amf0::error::{Amf0WriteError, Amf0WriteErrorValue};
use failure::{Backtrace, Fail};
use liverust_lib::netio::writer::IOWriteError;
use std::fmt;
use std::io;
use std::io::Error;
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
    IOError(Error),
    TimeoutError(Elapsed),

}

impl From<Error> for ServerError {
    fn from(error: Error) -> Self {
        ServerError {
            value: ServerErrorValue::IOError(error),
        }
    }
}

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
