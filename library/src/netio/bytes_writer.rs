use byteorder::{ByteOrder, WriteBytesExt};
use rand;
use rand::Rng;
use std::io;
use std::io::{Cursor, Write};

use super::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

use super::netio::NetworkIO;
use tokio::{prelude::*, stream::StreamExt, time::timeout};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub struct BytesWriter<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub bytes: Vec<u8>,
    pub io: Rc<RefCell<NetworkIO<S>>>,
}

impl<S> BytesWriter<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io: Rc<RefCell<NetworkIO<S>>>) -> Self {
        Self {
            bytes: Vec::new(),
            io: io,
        }
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
    pub async fn flush(&mut self) -> Result<(), BytesWriteError> {
        self.io
            .borrow_mut()
            .write(self.bytes.clone().into())
            .await?;
        self.bytes.clear();
        Ok(())
    }
}
