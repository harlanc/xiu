use super::bytesio_errors::{BytesIOError, BytesIOErrorValue};

use bytes::Bytes;
use bytes::BytesMut;

use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;

use tokio_stream::StreamExt;

use futures::SinkExt;
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
        let message = timeout(duration, self.stream.next()).await;

        match message {
            Ok(bytes) => {
                if let Some(data) = bytes {
                    match data {
                        Ok(bytes) => {
                            return Ok(bytes);
                        }
                        Err(err) => {
                            return Err(BytesIOError {
                                value: BytesIOErrorValue::IOError(err),
                            })
                        }
                    }
                } else {
                    return Err(BytesIOError {
                        value: BytesIOErrorValue::NoneReturn,
                    });
                }
            }
            Err(_) => {
                return Err(BytesIOError {
                    value: BytesIOErrorValue::TimeoutError,
                })
            }
        }
    }

    pub async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => {
                    // for k in bytes.clone(){
                    //     print!("{:02X} ",k);
                    // }
                    // print!("\n");
                    // print!("\n");
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

        // let data = self.framed_read.next().await;

        // match data {
        //     Some(result) => match result {
        //         Ok(bytes) => {
        //             return Ok(bytes);
        //         }
        //         Err(err) => {
        //             return Err(NetIOError {
        //                 value: NetIOErrorValue::IOError(err),
        //             })
        //         }
        //     },
        //     None => {
        //         return Err(NetIOError {
        //             value: NetIOErrorValue::NoneReturn,
        //         })
        //     }
        // }
    }
}
