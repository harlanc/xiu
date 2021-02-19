use failure::Fail;
use std::{io, string};

use liverust_lib::netio::{
    bytes_errors::{BytesReadError, BytesWriteError},
    bytes_reader::BytesReader,
    bytes_writer::BytesWriter,
};

//#[derive(Debug, Fail)]
pub enum Amf0ReadErrorValue {
    //#[fail(display = "Encountered unknown marker: {}", marker)]
    UnknownMarker { marker: u8 },

    //#[fail(display = "Unexpected empty object property name")]
    UnexpectedEmptyObjectPropertyName,

    //#[fail(display = "Hit end of the byte buffer but was expecting more data")]
    UnexpectedEof,

    //#[fail(display = "Failed to read byte buffer: {}", _0)]
    //BufferReadError(#[cause] io::Error),

    //#[fail(display = "Failed to read a utf8 string from the byte buffer: {}", _0)]
    StringParseError(string::FromUtf8Error),

    //#[fail(display = "Failed to read a utf8 string from the byte buffer: {}", _0)]
    BytesReadError(BytesReadError),
    WrongType,
}

pub struct Amf0ReadError {
    pub value: Amf0ReadErrorValue,
}

// Since an IO error can only be thrown while reading the buffer, auto-conversion should work
// impl From<io::Error> for Amf0ReadError {
//     fn from(error: io::Error) -> Self {
//         Amf0ReadError::BufferReadError(error)
//     }
// }

impl From<string::FromUtf8Error> for Amf0ReadError {
    fn from(error: string::FromUtf8Error) -> Self {
        Amf0ReadError {
            value: Amf0ReadErrorValue::StringParseError(error),
        }
    }
}

impl From<BytesReadError> for Amf0ReadError {
    fn from(error: BytesReadError) -> Self {
        Amf0ReadError {
            value: Amf0ReadErrorValue::BytesReadError(error),
        }
    }
}

// impl From<u8> for Amf0ReadError {
//     fn from(error: u8) -> Self {
//         Amf0ReadError {
//             value: Amf0ReadErrorValue::UnknownMarker(error),
//         }
//     }
// }

/// Errors raised during to the serialization process

pub enum Amf0WriteErrorValue {
    NormalStringTooLong,
    BufferWriteError(io::Error),
    BytesWriteError(BytesWriteError),
}

pub struct Amf0WriteError {
    pub value: Amf0WriteErrorValue,
}

impl From<io::Error> for Amf0WriteError {
    fn from(error: io::Error) -> Self {
        Amf0WriteError {
            value: Amf0WriteErrorValue::BufferWriteError(error),
        }
    }
}

impl From<BytesWriteError> for Amf0WriteError {
    fn from(error: BytesWriteError) -> Self {
        Amf0WriteError {
            value: Amf0WriteErrorValue::BytesWriteError(error),
        }
    }
}
