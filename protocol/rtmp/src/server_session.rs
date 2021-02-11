use super::errors::ServerError;
use crate::chunk::unpacketizer::ChunkUnpacketizer;
use crate::chunk::ChunkHeader;
use crate::handshake::handshake::SimpleHandshakeServer;
use crate::{chunk::packetizer::ChunkPacketizer, handshake};
use bytes::BytesMut;
use liverust_lib::netio::writer::{IOWriteError, Writer};
use std::{
    net::{TcpListener, TcpStream},
    slice::SplitMut,
    time::Duration,
};
use tokio::{
    prelude::*,
    stream::StreamExt,
    sync::{self, mpsc, oneshot},
    time::timeout,
};

enum ServerSessionState {
    Handshake,
    ReadChunk,
}

pub struct ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    //writer: Writer,
    packetizer: ChunkPacketizer,
    unpacketizer: ChunkUnpacketizer,
    handshaker: SimpleHandshakeServer,
    bytes_stream: tokio_util::codec::Framed<S, tokio_util::codec::BytesCodec>,
    state: ServerSessionState,
}

impl<S> ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(io_writer: Writer, stream: S, timeout: Duration) -> Self {
        let bytesMut = BytesMut::new();
        Self {
            //writer: io_writer,
            packetizer: ChunkPacketizer::new(io_writer),
            unpacketizer: ChunkUnpacketizer::new(BytesMut::new()),
            handshaker: SimpleHandshakeServer::new(BytesMut::new()),
            bytes_stream: tokio_util::codec::Framed::new(
                stream,
                tokio_util::codec::BytesCodec::new(),
            ),
            state: ServerSessionState::Handshake,
        }
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        let duration = Duration::new(10, 10);

        loop {
            let val = self.bytes_stream.try_next();
            match timeout(duration, val).await? {
                Ok(Some(data)) => match self.state {
                    ServerSessionState::Handshake => {
                        let result = self.handshaker.handshake();
                        match result {
                            Ok(v) => {
                                self.state = ServerSessionState::ReadChunk;
                            }
                            Err(e) => {}
                        }
                    }
                    ServerSessionState::ReadChunk => {
                        let result = self.unpacketizer.read_chunk(&data[..])?;
                    }
                },
                _ => {}
            }
        }

        Ok(())
    }
    fn send_control() {

        // struct rtmp_chunk_header_t header;
        // header.fmt = RTMP_CHUNK_TYPE_0; // disable compact header
        // header.cid = RTMP_CHANNEL_INVOKE;
        // header.timestamp = 0;
        // header.length = bytes;
        // header.type = RTMP_TYPE_INVOKE;
        // header.stream_id = stream_id; /* default 0 */
        // return rtmp_chunk_write(rtmp, &header, payload);
    }
}
