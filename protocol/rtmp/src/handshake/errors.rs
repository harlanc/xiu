
use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use hmac::{Hmac, Mac};
use rand;
use rand::Rng;
use sha2::Sha256;
use std::convert::TryInto;
use std::io::{Cursor, Write};
use std::{collections::HashMap, ops::BitOr};
use tokio_util::codec::{BytesCodec, Framed};

use netio::{
    bytes_errors::{BytesReadError, BytesWriteError},
    //bytes_reader::NetworkReader,
    bytes_reader::BytesReader,
    bytes_writer::AsyncBytesWriter,
    netio::NetworkIO,
};

use tokio::prelude::*;

use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::time::{SystemTime, SystemTimeError};

pub enum HandshakeErrorValue {
    BytesReadError(BytesReadError),
    BytesWriteError(BytesWriteError),
    SysTimeError(SystemTimeError),
    DigestNotFound,
    S0VersionNotCorrect,
}

pub struct HandshakeError {
    pub value: HandshakeErrorValue,
}

impl From<HandshakeErrorValue> for HandshakeError {
    fn from(val: HandshakeErrorValue) -> Self {
        HandshakeError { value: val }
    }
}

impl From<BytesReadError> for HandshakeError {
    fn from(error: BytesReadError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for HandshakeError {
    fn from(error: BytesWriteError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::BytesWriteError(error),
        }
    }
}

impl From<SystemTimeError> for HandshakeError {
    fn from(error: SystemTimeError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::SysTimeError(error),
        }
    }
}