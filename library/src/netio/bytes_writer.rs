use byteorder::{ByteOrder, WriteBytesExt};
use bytes::BytesMut;
use rand;
use rand::Rng;

use std::io::Write;

use super::bytes_errors::BytesWriteError;

use super::netio::NetworkIO;
use tokio::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

pub struct BytesWriter {
    pub bytes: Vec<u8>,
}

impl BytesWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), BytesWriteError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), BytesWriteError> {
        self.bytes.write_u16::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u24::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), BytesWriteError> {
        self.bytes.write_f64::<T>(bytes)?;
        Ok(())
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        self.bytes.write(buf)?;
        Ok(())
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), BytesWriteError> {
        let mut rng = rand::thread_rng();
        for _ in 0..length {
            self.bytes.write_u8(rng.gen())?;
        }
        Ok(())
    }
    pub fn extract_current_bytes(&mut self) -> BytesMut {
        let mut rv_data = BytesMut::new();
        rv_data.extend_from_slice(&self.bytes.clone()[..]);
        self.bytes.clear();

        rv_data
    }
}

pub struct AsyncBytesWriter<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub bytes_writer: BytesWriter,
    pub io: Rc<RefCell<NetworkIO<S>>>,
}

impl<S> AsyncBytesWriter<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io: Rc<RefCell<NetworkIO<S>>>) -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            io: io,
        }
    }

    pub async fn flush(&mut self) -> Result<(), BytesWriteError> {
        self.io
            .borrow_mut()
            .write(self.bytes_writer.bytes.clone().into())
            .await?;
        self.bytes_writer.bytes.clear();
        Ok(())
    }
}
