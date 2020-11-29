use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use std::collections::HashMap;
use std::io;
use std::io::{Cursor, Write};
use std::time::SystemTime;

const RTMP_VERSION: usize = 3;
const RTMP_HANDSHAKE_SIZE: usize = 1536;

pub enum HandshakeErrorValue {
    IO(io::Error),
}

pub struct HandshakeError {
    pub value: HandshakeErrorValue,
}

impl From<HandshakeErrorValue> for HandshakeError {
    fn from(val: HandshakeErrorValue) -> Self {
        HandshakeError { value: val }
    }
}

impl From<io::Error> for HandshakeError {
    fn from(error: io::Error) -> Self {
        HandshakeError {
            value: HandshakeErrorValue::IO(error),
        }
    }
}
pub struct SimpleHandshake {
    bytes: Cursor<Vec<u8>>,
}

impl SimpleHandshake {
    fn write_u8(&mut self, byte: u8) -> Result<(), HandshakeError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    fn write_u16(&mut self, bytes: u16) -> Result<(), HandshakeError> {
        self.bytes.write_u16::<BigEndian>(bytes)?;
        Ok(())
    }

    fn write_u24(&mut self, bytes: u32) -> Result<(), HandshakeError> {
        self.bytes.write_u24::<BigEndian>(bytes)?;
        Ok(())
    }

    fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), HandshakeError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), HandshakeError> {
        self.bytes.write(buf)?;
        Ok(())
    }
    fn write_c0(&mut self) -> Result<(), HandshakeError> {
        self.write_u8(RTMP_VERSION as u8)?;
        Ok(())
    }

    fn write_c1(&mut self) {
        let nano_secs = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => println!("1970-01-01 00:00:00 UTC was {} seconds ago!", n.as_nanos()),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };
    }
}
