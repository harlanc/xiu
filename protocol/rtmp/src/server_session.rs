use super::define;
use super::errors::ServerError;
use super::errors::ServerErrorValue;
use crate::handshake::handshake::SimpleHandshakeServer;
use crate::{amf0::Amf0ValueType, chunk::unpacketizer::UnpackResult};
use crate::{chunk::packetizer::ChunkPacketizer, handshake};
use crate::{
    chunk::{
        unpacketizer::{self, ChunkUnpacketizer},
        ChunkInfo,
    },
    netconnection,
};
use crate::{
    chunk::{Chunk, ChunkHeader},
    netstream,
};

use crate::messages::define::Rtmp_Messages;
use crate::messages::processor::MessageProcessor;
use bytes::BytesMut;
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

                        match result {
                            UnpackResult::ChunkInfo(chunk_info) => {
                                let mut message_parser = MessageProcessor::new(chunk_info);
                                let mut rtmp_msg = message_parser.execute()?;

                                self.process_rtmp_message(&mut rtmp_msg)?;
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
    pub fn process_rtmp_message(
        &mut self,
        rtmp_msg: &mut Rtmp_Messages,
    ) -> Result<(), ServerError> {
        match rtmp_msg {
            Rtmp_Messages::AMF0_COMMAND {
                msg_stream_id,
                command_name,
                transaction_id,
                command_object,
                others,
            } => self.process_amf0_command_message(
                msg_stream_id,
                command_name,
                transaction_id,
                command_object,
                others,
            )?,

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

    pub fn process_amf0_command_message(
        &mut self,
        stream_id: &u32,
        command_name: &Amf0ValueType,
        transaction_id: &Amf0ValueType,
        command_object: &Amf0ValueType,
        others: &mut Vec<Amf0ValueType>,
    ) -> Result<(), ServerError> {
        let empty_cmd_name = &String::new();
        let cmd_name = match command_name {
            Amf0ValueType::UTF8String(str) => str,
            _ => empty_cmd_name,
        };

        let transaction_id = match transaction_id {
            Amf0ValueType::Number(number) => number,
            _ => &0.0,
        };

        let empty_cmd_obj: HashMap<String, Amf0ValueType> = HashMap::new();
        let obj = match command_object {
            Amf0ValueType::Object(obj) => obj,
            _ => &empty_cmd_obj,
        };

        match cmd_name.as_str() {
            "connect" => {
                self.on_connect(&transaction_id, &obj)?;
            }
            "createStream" => {
                self.on_create_stream(transaction_id)?;
            }
            "deleteStream" => {
                if others.len() > 1 {
                    let stream_id = match others.pop() {
                        Some(val) => match val {
                            Amf0ValueType::Number(streamid) => streamid,
                            _ => 0.0,
                        },
                        _ => 0.0,
                    };

                    self.on_delete_stream(transaction_id, &stream_id)?;
                }
            }
            "play" => {
                self.on_play(transaction_id, stream_id, others)?;
            }
            "publish" => {
                self.on_publish(transaction_id, stream_id, others)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn on_connect(
        &mut self,
        transaction_id: &f64,
        command_obj: &HashMap<String, Amf0ValueType>,
    ) -> Result<(), ServerError> {
        let mut control_message = ControlMessages::new(Writer::new());
        control_message.write_window_acknowledgement_size(define::WINDOW_ACKNOWLEDGEMENT_SIZE)?;
        control_message.write_set_peer_bandwidth(
            define::PEER_BANDWIDTH,
            define::PeerBandWidthLimitType::DYNAMIC,
        )?;
        control_message.write_set_chunk_size(define::CHUNK_SIZE)?;

        let obj_encoding = command_obj.get("objectEncoding");
        let encoding = match obj_encoding {
            Some(Amf0ValueType::Number(encoding)) => encoding,
            _ => &define::OBJENCODING_AMF0,
        };

        let mut netconnection = NetConnection::new(Writer::new());
        netconnection.connect_reply(
            &transaction_id,
            &define::FMSVER.to_string(),
            &define::CAPABILITIES,
            &String::from("NetConnection.Connect.Success"),
            &define::LEVEL.to_string(),
            &String::from("Connection Succeeded."),
            encoding,
        )?;
        Ok(())
    }

    pub fn on_create_stream(&mut self, transaction_id: &f64) -> Result<(), ServerError> {
        let mut netconnection = NetConnection::new(Writer::new());
        netconnection.create_stream_reply(transaction_id, &define::STREAM_ID)?;

        Ok(())
    }

    pub fn on_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), ServerError> {
        let mut netstream = NetStream::new(Writer::new());
        netstream.on_status(
            transaction_id,
            &"status".to_string(),
            &"NetStream.DeleteStream.Suceess".to_string(),
            &"".to_string(),
        )?;

        Ok(())
    }
    pub fn on_play(
        &mut self,
        transaction_id: &f64,
        stream_id: &u32,
        other_values: &mut Vec<Amf0ValueType>,
    ) -> Result<(), ServerError> {
        let length = other_values.len() as u8;
        let mut index: u8 = 0;

        let mut stream_name: Option<String> = None;
        let mut start: Option<f64> = None;
        let mut duration: Option<f64> = None;
        let mut reset: Option<bool> = None;

        loop {
            if index >= length {
                break;
            }
            index = index + 1;
            stream_name = match other_values.remove(0) {
                Amf0ValueType::UTF8String(val) => Some(val),
                _ => None,
            };

            if index >= length {
                break;
            }
            index = index + 1;
            start = match other_values.remove(0) {
                Amf0ValueType::Number(val) => Some(val),
                _ => None,
            };

            if index >= length {
                break;
            }
            index = index + 1;
            duration = match other_values.remove(0) {
                Amf0ValueType::Number(val) => Some(val),
                _ => None,
            };

            if index >= length {
                break;
            }
            index = index + 1;
            reset = match other_values.remove(0) {
                Amf0ValueType::Boolean(val) => Some(val),
                _ => None,
            };
            break;
        }

        let mut event_messages = EventMessages::new(Writer::new());
        event_messages.stream_begin(stream_id.clone())?;

        let mut netstream = NetStream::new(Writer::new());
        match reset {
            Some(val) => {
                if val {
                    netstream.on_status(
                        transaction_id,
                        &"status".to_string(),
                        &"NetStream.Play.Reset".to_string(),
                        &"".to_string(),
                    )?;
                }
            }
            _ => {}
        }

        netstream.on_status(
            transaction_id,
            &"status".to_string(),
            &"NetStream.Play.Start".to_string(),
            &"".to_string(),
        )?;

        event_messages.stream_is_record(stream_id.clone())?;

        Ok(())
    }

    pub fn on_publish(
        &mut self,
        transaction_id: &f64,
        stream_id: &u32,
        other_values: &mut Vec<Amf0ValueType>,
    ) -> Result<(), ServerError> {
        let length = other_values.len();

        if length < 2 {
            return Err(ServerError {
                value: ServerErrorValue::Amf0ValueCountNotCorrect,
            });
        }

        let stream_name = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(ServerError {
                    value: ServerErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        let stream_type = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(ServerError {
                    value: ServerErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        let mut event_messages = EventMessages::new(Writer::new());
        event_messages.stream_begin(stream_id.clone())?;

        let mut netstream = NetStream::new(Writer::new());
        netstream.on_status(
            transaction_id,
            &"status".to_string(),
            &"NetStream.Publish.Start".to_string(),
            &"".to_string(),
        )?;

        Ok(())
    }
}
