use super::bytesio_errors::{BytesIOError, BytesIOErrorValue};

use bytes::Bytes;
use bytes::BytesMut;

use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

use tokio_stream::StreamExt;

use futures::SinkExt;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

pub struct BytesIO {
    stream: Framed<TcpStream, BytesCodec>,
    //timeout: Duration,
}

impl BytesIO {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Framed::new(stream, BytesCodec::new()),
            // timeout: ms,
        }
    }

    pub async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.stream.send(bytes).await?;
        Ok(())
    }

    pub async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        let begin_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        loop {
            match self.read().await {
                Ok(data) => {
                    return Ok(data);
                }
                Err(_) => {
                    sleep(Duration::from_millis(50)).await;
                    let current_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                    if current_millseconds - begin_millseconds > duration {
                        return Err(BytesIOError {
                            value: BytesIOErrorValue::TimeoutError,
                        });
                    }
                }
            }
        }
    }

    pub async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => {
                    return Ok(bytes);
                }
                Err(err) => {
                    return Err(BytesIOError {
                        value: BytesIOErrorValue::IOError(err),
                    })
                }
            },
            None => {
                return Err(BytesIOError {
                    value: BytesIOErrorValue::NoneReturn,
                })
            }
        }
    }
}
