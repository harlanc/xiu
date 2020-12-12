use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use rand;
use rand::Rng;
use std::{collections::HashMap, ops::BitOr};

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
    s1_bytes: BytesMut,
}

fn current_time() -> u32 {
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match duration {
        Ok(result) => result.as_nanos() as u32,
        _ => 0,
    }
}

impl SimpleHandshakeClient {
    fn write_c0(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }
    fn write_c1(&mut self) -> Result<(), HandshakeError> {
        self.writer.write_u32::<BigEndian>(current_time())?;
        self.writer.write_u32::<BigEndian>(0)?;

        let mut rng = rand::thread_rng();
        for x in 0..(RTMP_HANDSHAKE_SIZE - 8) {
            self.writer.write_u8(rng.gen())?;
        }
        Ok(())
    }

    fn write_c2(&mut self) -> Result<(), HandshakeError> {
        //let time = self.s1_bytes.split_to(4);
        self.writer.write(&self.s1_bytes[0..])?;
        self.writer.write_u32::<BigEndian>(current_time())?;
        Ok(())
    }

    fn read_s0(&mut self) -> Result<(), HandshakeError> {
        self.reader.read_u8()?;
        Ok(())
    }
    fn read_s1(&mut self) -> Result<(), HandshakeError> {
        self.s1_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }
    fn read_s2(&mut self) -> Result<(), HandshakeError> {
        let s2_bytes = self.reader.read_bytes(RTMP_HANDSHAKE_SIZE)?;
        Ok(())
    }
}
