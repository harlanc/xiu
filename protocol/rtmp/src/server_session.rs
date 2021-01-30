use super::errors::ServerError;
use crate::chunk::packetizer::ChunkPacketizer;
use crate::chunk::unpacketizer::ChunkUnpacketizer;
use crate::chunk::ChunkHeader;
use bytes::BytesMut;
use liverust_lib::netio::writer::{IOWriteError, Writer};
use std::{
    net::{TcpListener, TcpStream},
    time::Duration,
};
use tokio::{
    prelude::*,
    stream::StreamExt,
    sync::{self, mpsc, oneshot},
    time::timeout,
    
};
// use tokio_util::codec::Framed;
// use tokio_util::codec::BytesCodec;

pub struct ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    //writer: Writer,
    packetizer: ChunkPacketizer,
    unpacketizer: ChunkUnpacketizer,
    bytes_stream: tokio_util::codec::Framed<S, tokio_util::codec::BytesCodec>,
}

impl<S> ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(io_writer: Writer, stream: S) -> Self {
        let bytesMut = BytesMut::new();
        Self {
            //writer: io_writer,
            packetizer: ChunkPacketizer::new(io_writer),
            unpacketizer: ChunkUnpacketizer::new(bytesMut),
            bytes_stream: tokio_util::codec::Framed::new(
                stream,
                tokio_util::codec::BytesCodec::new(),
            ),
        }
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        let duration = Duration::new(10, 10);
        let val = self.bytes_stream.try_next();
        match timeout(duration, val).await? {
            Ok(Some(data)) => {
                // for event in self.proto.handle_bytes(&data).unwrap() {
                //     self.handle_event(event).await?;
                // }
            }
            _ => {}
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
