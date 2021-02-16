use super::define;
use super::errors::ClientError;
use super::errors::ClientErrorValue;
use crate::chunk::{
    unpacketizer::{self, ChunkUnpacketizer},
    ChunkInfo,
};
use crate::{amf0::Amf0ValueType, chunk::unpacketizer::UnpackResult};
use crate::{chunk::packetizer::ChunkPacketizer, handshake};
use crate::{
    chunk::{Chunk, ChunkHeader},
    netstream,
};
use crate::{handshake::handshake::SimpleHandshakeClient, netconnection};

use crate::messages::define::Rtmp_Messages;
use crate::messages::processor::MessageProcessor;
use bytes::BytesMut;
use handshake::handshake::SimpleHandshakeServer;
use liverust_lib::netio::errors::IOWriteError;
use liverust_lib::netio::writer::Writer;
use std::{
    net::{TcpListener, TcpStream},
    slice::SplitMut,
    time::Duration,
};

use crate::netconnection::commands::NetConnection;
use crate::netstream::commands::NetStream;
use crate::protocol_control_messages::control_messages::ControlMessages;
use crate::user_control_messages::errors::EventMessagesError;
use crate::user_control_messages::event_messages::EventMessages;

use std::collections::HashMap;

use tokio::{
    prelude::*,
    stream::StreamExt,
    sync::{self, mpsc, oneshot},
    time::timeout,
};
use tokio_util::codec::{BytesCodec, Framed};
pub struct ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    //writer: Writer,
    packetizer: ChunkPacketizer,
    unpacketizer: ChunkUnpacketizer,
    handshaker: SimpleHandshakeClient<S>,
    bytes_stream: Framed<S, BytesCodec>,
    state: ClientSessionState,
}

enum ClientSessionState {
    Handshake,
    ReadChunk,
}

impl<S> ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(io_writer: Writer, stream: S, timeout: Duration) -> Self {
        let bytesMut = BytesMut::new();
        let stream = Framed::new(stream, BytesCodec::new());
        Self {
            //writer: io_writer,
            packetizer: ChunkPacketizer::new(io_writer),
            unpacketizer: ChunkUnpacketizer::new(BytesMut::new()),
            bytes_stream: stream,
            handshaker: SimpleHandshakeClient::new(BytesMut::new(), Writer::new(), stream),
            state: ClientSessionState::Handshake,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        let duration = Duration::new(10, 10);

        loop {
            let val = self.bytes_stream.try_next();
            match timeout(duration, val).await? {
                Ok(Some(data)) => match self.state {
                    ClientSessionState::Handshake => {
                        let result = self.handshaker.handshake().await;
                        match result {
                            Ok(_) => {
                                //let netconnection = NetConnection::new(Writer::new());
                                //netconnection.connect_reply(transaction_id, fmsver, capabilities, code, level, description, encoding)
                                self.state = ClientSessionState::ReadChunk;
                            }
                            Err(_) => {}
                        }
                    }
                    ClientSessionState::ReadChunk => {
                        let result = self.unpacketizer.read_chunk(&data[..])?;

                        match result {
                            UnpackResult::ChunkInfo(chunk_info) => {
                                let mut message_parser = MessageProcessor::new(chunk_info);
                                let mut rtmp_msg = message_parser.execute()?;

                                // self.process_rtmp_message(&mut rtmp_msg)?;
                            }
                            _ => {}
                        }
                    }
                },
                _ => {}
            }
        }

        Ok(())
    }
}
