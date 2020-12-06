use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use rand;
use rand::Rng;
use std::collections::HashMap;


use liverust_lib::netio::{
    reader::{IOReadError, Reader},
    writer::{IOWriteError, Writer},
};
use std::time::{SystemTime, SystemTimeError};

const RTMP_VERSION: usize = 3;
const RTMP_HANDSHAKE_SIZE: usize = 1536;

pub enum HandshakeErrorValue {
    IORead(IOReadError),
    IOWrite(IOWriteError),
    SysTimeError(SystemTimeError),
}

pub struct HandshakeError {
    pub value: HandshakeErrorValue,
}

impl From<HandshakeErrorValue> for HandshakeError {
    fn from(val: HandshakeErrorValue) -> Self {
        HandshakeError { value: val }
    }
}

impl From<IOReadError> for HandshakeError {
    fn from(error: IOReadError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IORead(error),
        }
    }
}

impl From<IOWriteError> for HandshakeError {
    fn from(error: IOWriteError) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IOWrite(error),
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
pub struct SimpleHandshakeClient {
    // bytes: Cursor<Vec<u8>>,
    // buffer: BytesMut,
    reader: Reader,
    writer: Writer,
}

impl SimpleHandshakeClient {
    fn write_c1(&mut self) -> Result<(), HandshakeError> {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        self.writer
            .write_u32::<BigEndian>(duration.as_nanos() as u32)?;
        self.writer.write_u32::<BigEndian>(0)?;

        let mut rng = rand::thread_rng();
        for x in 0..(RTMP_HANDSHAKE_SIZE - 8) {
            self.writer.write_u8(rng.gen())?;
        }
        Ok(())
    }

    fn write_c2(&mut self) -> Result<(), HandshakeError> {
        Ok(())
    }

    fn read_s0(&mut self) -> Result<(), HandshakeError> {
        Ok(())
    }
}
