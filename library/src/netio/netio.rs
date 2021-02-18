use super::bytes_reader::BytesReader;
use super::bytes_writer::BytesWriter;
use super::netio_errors::{NetIOError, NetIOErrorValue};
use byteorder::{ByteOrder, ReadBytesExt};
use bytes::Bytes;
use bytes::BytesMut;
use futures::SinkExt;
use std::io::Cursor;
use std::time::Duration;
use std::{convert::TryInto, io};

use tokio::{prelude::*, stream::StreamExt, time::timeout};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

pub struct NetworkIO<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    bytes_stream: Framed<S, BytesCodec>,
    timeout: Duration,
}

impl<S> NetworkIO<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: S, ms: Duration) -> Self {
        Self {
            bytes_stream: Framed::new(stream, BytesCodec::new()),
            timeout: ms,
        }
    }

    pub async fn write(&mut self, bytes: Bytes) -> Result<(), NetIOError> {
        self.bytes_stream.send(bytes).await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<BytesMut, NetIOError> {
        let val = self.bytes_stream.try_next();
        match timeout(self.timeout, val).await? {
            Ok(Some(data)) => {
                return Ok(data);
            }
            Ok(None) => {
                return Err(NetIOError {
                    value: NetIOErrorValue::NoneReturn,
                })
            }
            Err(err) => {
                return Err(NetIOError {
                    value: NetIOErrorValue::IOError(err),
                })
            }
        }
    }
}
