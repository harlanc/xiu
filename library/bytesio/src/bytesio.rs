use std::net::SocketAddr;

use std::time::Duration;
use async_trait::async_trait;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use futures::SinkExt;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

use super::bytesio_errors::{BytesIOError, BytesIOErrorValue};

pub enum NetType {
    TCP,
    UDP,
}

#[async_trait]
pub trait TNetIO: Send + Sync {
    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
    async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
    async fn read_min_bytes_with_timeout(
        &mut self,
        duration: Duration,
        size: usize
    ) -> Result<BytesMut, BytesIOError>;
    fn get_net_type(&self) -> NetType;
}

pub struct UdpIO {
    socket: UdpSocket,
}

impl UdpIO {
    pub async fn new(remote_domain: String, remote_port: u16, local_port: u16) -> Option<Self> {
        let remote_address = if remote_domain == "localhost" {
            format!("127.0.0.1:{remote_port}")
        } else {
            format!("{remote_domain}:{remote_port}")
        };
        log::info!("remote address: {}", remote_address);
        let local_address = format!("0.0.0.0:{local_port}");
        if let Ok(local_socket) = UdpSocket::bind(local_address).await {
            if let Ok(remote_socket_addr) = remote_address.parse::<SocketAddr>() {
                if let Err(err) = local_socket.connect(remote_socket_addr).await {
                    log::info!("connect to remote udp socket error: {}", err);
                }

                return Some(Self {
                    socket: local_socket,
                });
            } else {
                log::error!("remote_address parse error: {:?}", remote_address);
            }
        }

        None
    }
    pub fn get_local_port(&self) -> Option<u16> {
        if let Ok(local_addr) = self.socket.local_addr() {
            log::info!("local address: {}", local_addr);
            return Some(local_addr.port());
        }

        None
    }
}

#[async_trait]
impl TNetIO for UdpIO {
    fn get_net_type(&self) -> NetType {
        NetType::UDP
    }

    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.socket.send(bytes.as_ref()).await?;
        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    // As udp is based on packet, we can't read part of them, so the size parameter is not effective
    async fn read_min_bytes_with_timeout(
        &mut self,
        duration: Duration,
        _size: usize
    ) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let mut buf = vec![0; 4096];
        let len = self.socket.recv(&mut buf).await?;
        let mut rv = BytesMut::new();
        rv.put(&buf[..len]);

        Ok(rv)
    }
}

pub struct TcpIO {
    stream: Framed<TcpStream, BytesCodec>,
    //timeout: Duration,
}

impl TcpIO {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Framed::new(stream, BytesCodec::new()),
            // timeout: ms,
        }
    }
}

#[async_trait]
impl TNetIO for TcpIO {
    fn get_net_type(&self) -> NetType {
        NetType::TCP
    }

    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.stream.send(bytes).await?;

        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read_min_bytes_with_timeout(
        &mut self,
        duration: Duration,
        size: usize
    ) -> Result<BytesMut, BytesIOError> {
        let mut result: BytesMut = BytesMut::new();
        let start_time = tokio::time::Instant::now();
        loop {
            let current_time = tokio::time::Instant::now();
            let remaining_duration = match duration.checked_sub(current_time - start_time) {
                Some(remaining) => remaining,
                None => Duration::from_secs(0),
            };
            let message_result = tokio::time::timeout(remaining_duration, self.stream.next()).await;
            match message_result {
                Ok(message) => match message {
                    Some(data) => match data {
                        Ok(bytes) => {
                            result.extend_from_slice(&bytes);
                            if result.len() >= size {
                                return Ok(result);
                            }
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
                },
                Err(err) => {
                    return Err(BytesIOError {
                        value: BytesIOErrorValue::TimeoutError(err),
                    })
                }
            }
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => Ok(bytes),
                Err(err) => Err(BytesIOError {
                    value: BytesIOErrorValue::IOError(err),
                }),
            },
            None => Err(BytesIOError {
                value: BytesIOErrorValue::NoneReturn,
            }),
        }
    }
}
