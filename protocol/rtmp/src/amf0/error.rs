use std::{io, string};

#[derive(Debug, Fail)]
pub enum Amf0ReadError {
    #[fail(display = "Encountered unknown marker: {}", marker)]
    UnknownMarker { marker: u8 },

    #[fail(display = "Unexpected empty object property name")]
    UnexpectedEmptyObjectPropertyName,

    #[fail(display = "Hit end of the byte buffer but was expecting more data")]
    UnexpectedEof,

    #[fail(display = "Failed to read byte buffer: {}", _0)]
    BufferReadError(#[cause] io::Error),

    #[fail(display = "Failed to read a utf8 string from the byte buffer: {}", _0)]
    StringParseError(#[cause] string::FromUtf8Error),
}

// Since an IO error can only be thrown while reading the buffer, auto-conversion should work
impl From<io::Error> for Amf0ReadError {
    fn from(error: io::Error) -> Self {
        Amf0ReadError::BufferReadError(error)
    }
}

impl From<string::FromUtf8Error> for Amf0ReadError {
    fn from(error: string::FromUtf8Error) -> Self {
        Amf0ReadError::StringParseError(error)
    }
}

/// Errors raised during to the serialization process
#[derive(Debug, Fail)]
pub enum Amf0WriteError {
    #[fail(display = "String length greater than 65,535")]
    NormalStringTooLong,

    #[fail(display = "Failed to write to byte buffer")]
    BufferWriteError(#[cause] io::Error),
}

impl From<io::Error> for Amf0WriteError {
    fn from(error: io::Error) -> Self {
        Amf0WriteError::BufferWriteError(error)
    }
}
