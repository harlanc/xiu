use byteorder::{ ByteOrder, WriteBytesExt};
use rand;
use rand::Rng;
use std::io;
use std::io::{Cursor, Write};

use super::errors::{IOWriteError, IOWriteErrorValue};


pub struct Writer {
    bytes: Cursor<Vec<u8>>,
}

impl Writer {
    pub fn new() -> Writer {
        Writer { bytes: Cursor::new(Vec::new()) }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), IOWriteError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), IOWriteError> {
        self.bytes.write_u16::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), IOWriteError> {
        self.bytes.write_u24::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), IOWriteError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), IOWriteError> {
        self.bytes.write_f64::<T>(bytes)?;
        Ok(())
    }
    
    pub fn write(&mut self, buf: &[u8]) -> Result<(), IOWriteError> {
        self.bytes.write(buf)?;
        Ok(())
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), IOWriteError> {
        let mut rng = rand::thread_rng();
        for _ in 0..length {
            self.bytes.write_u8(rng.gen())?;
        }
        Ok(())
    }
}
