use super::define;
use super::errors::SessionError;
use super::errors::SessionErrorValue;
use crate::chunk::packetizer::ChunkPacketizer;
use crate::chunk::{unpacketizer::ChunkUnpacketizer, ChunkInfo};
use crate::handshake::handshake::SimpleHandshakeServer;
use crate::{amf0::Amf0ValueType, chunk::unpacketizer::UnpackResult};
use crate::{
    chunk::define::CHUNK_SIZE,
    chunk::define::{chunk_type, csid_type},
};

use crate::messages::define::msg_type_id;
use crate::messages::define::MessageTypes;

use crate::messages::parser::MessageParser;
use bytes::BytesMut;

use netio::bytes_writer::AsyncBytesWriter;
use netio::bytes_writer::BytesWriter;
use netio::netio::NetworkIO;
use std::time::Duration;

use crate::netconnection::commands::NetConnection;
use crate::netstream::commands::NetStream;
use crate::protocol_control_messages::control_messages::ControlMessages;

use crate::user_control_messages::event_messages::EventMessages;

use std::collections::HashMap;

use tokio::prelude::*;

// use std::cell::{RefCell, RefMut};
// use std::rc::Rc;

use std::sync::Arc;
use tokio::sync::Mutex;

enum ServerSessionState {
    Handshake,
    ReadChunk,
    // OnConnect,
    // OnCreateStream,
    // OnPlay,
    // OnPublish,
}

pub struct ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    packetizer: ChunkPacketizer<S>,
    unpacketizer: ChunkUnpacketizer,
    handshaker: SimpleHandshakeServer<S>,

    io: Arc<Mutex<NetworkIO<S>>>,
    state: ServerSessionState,
}

impl<S> ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pub fn new(stream: S, timeout: Duration) -> Self {
        let net_io = Arc::new(Mutex::new(NetworkIO::new(stream, timeout)));
        let bytes_writer = AsyncBytesWriter::new(Arc::clone(&net_io));

        Self {
            packetizer: ChunkPacketizer::new(bytes_writer),
            unpacketizer: ChunkUnpacketizer::new(),
            handshaker: SimpleHandshakeServer::new(Arc::clone(&net_io)),

            io: Arc::clone(&net_io),
            state: ServerSessionState::Handshake,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        let duration = Duration::new(10, 10);

        loop {
            let data = self.io.lock().await.read().await?;

            match self.state {
                ServerSessionState::Handshake => {
                    self.handshaker.extend_data(&data[..]);
                    let result = self.handshaker.handshake().await;

                    match result {
                        Ok(v) => {
                            self.state = ServerSessionState::ReadChunk;
                        }
                        Err(e) => {}
                    }
                }
                ServerSessionState::ReadChunk => {
                    self.unpacketizer.extend_data(&data[..]);
                    let result = self.unpacketizer.read_chunk()?;

                    match result {
                        UnpackResult::ChunkInfo(chunk_info) => {
                            let msg_stream_id = chunk_info.message_header.msg_streamd_id;
                            let mut message_parser = MessageParser::new(chunk_info);
                            let mut msg = message_parser.parse()?;

                            self.process_messages(&mut msg, &msg_stream_id).await?;
                        }
                        _ => {}
                    }
                }
            }
        }

        //Ok(())
    }
    pub fn send_set_chunk_size(&mut self) -> Result<(), SessionError> {
        let mut controlmessage = ControlMessages::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_set_chunk_size(CHUNK_SIZE)?;

        Ok(())
    }
    pub async fn send_audio(&mut self, data: BytesMut) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::AUDIO,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::AUDIO,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub async fn send_video(&mut self, data: BytesMut) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::VIDEO,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::VIDEO,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }
    pub async fn process_messages(
        &mut self,
        rtmp_msg: &mut MessageTypes,
        msg_stream_id: &u32,
    ) -> Result<(), SessionError> {
        match rtmp_msg {
            MessageTypes::Amf0Command {
                command_name,
                transaction_id,
                command_object,
                others,
            } => {
                self.process_amf0_command_message(
                    msg_stream_id,
                    command_name,
                    transaction_id,
                    command_object,
                    others,
                )
                .await?
            }

            _ => {}
        }
        Ok(())
    }

    pub async fn process_amf0_command_message(
        &mut self,
        stream_id: &u32,
        command_name: &Amf0ValueType,
        transaction_id: &Amf0ValueType,
        command_object: &Amf0ValueType,
        others: &mut Vec<Amf0ValueType>,
    ) -> Result<(), SessionError> {
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
                self.on_connect(&transaction_id, &obj).await?;
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

                    self.on_delete_stream(transaction_id, &stream_id).await?;
                }
            }
            "play" => {
                self.on_play(transaction_id, stream_id, others).await?;
            }
            "publish" => {
                self.on_publish(transaction_id, stream_id, others).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn on_connect(
        &mut self,
        transaction_id: &f64,
        command_obj: &HashMap<String, Amf0ValueType>,
    ) -> Result<(), SessionError> {
        let mut control_message = ControlMessages::new(AsyncBytesWriter::new(self.io.clone()));
        control_message.write_window_acknowledgement_size(define::WINDOW_ACKNOWLEDGEMENT_SIZE)?;
        control_message.write_set_peer_bandwidth(
            define::PEER_BANDWIDTH,
            define::PeerBandWidthLimitType::DYNAMIC,
        )?;
        control_message.write_set_chunk_size(CHUNK_SIZE)?;

        let obj_encoding = command_obj.get("objectEncoding");
        let encoding = match obj_encoding {
            Some(Amf0ValueType::Number(encoding)) => encoding,
            _ => &define::OBJENCODING_AMF0,
        };

        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection.connect_response(
            &transaction_id,
            &define::FMSVER.to_string(),
            &define::CAPABILITIES,
            &String::from("NetConnection.Connect.Success"),
            &define::LEVEL.to_string(),
            &String::from("Connection Succeeded."),
            encoding,
        )?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub fn on_create_stream(&mut self, transaction_id: &f64) -> Result<(), SessionError> {
        let mut netconnection = NetConnection::new(BytesWriter::new());
        netconnection.create_stream_response(transaction_id, &define::STREAM_ID)?;

        Ok(())
    }

    pub async fn on_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), SessionError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        let data = netstream.on_status(
            transaction_id,
            &"status".to_string(),
            &"NetStream.DeleteStream.Suceess".to_string(),
            &"".to_string(),
        )?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }
    pub async fn on_play(
        &mut self,
        transaction_id: &f64,
        stream_id: &u32,
        other_values: &mut Vec<Amf0ValueType>,
    ) -> Result<(), SessionError> {
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

        let mut event_messages = EventMessages::new(AsyncBytesWriter::new(self.io.clone()));
        event_messages.stream_begin(stream_id.clone()).await?;

        let mut netstream = NetStream::new(BytesWriter::new());
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

        event_messages.stream_is_record(stream_id.clone()).await?;

        Ok(())
    }

    pub async fn on_publish(
        &mut self,
        transaction_id: &f64,
        stream_id: &u32,
        other_values: &mut Vec<Amf0ValueType>,
    ) -> Result<(), SessionError> {
        let length = other_values.len();

        if length < 2 {
            return Err(SessionError {
                value: SessionErrorValue::Amf0ValueCountNotCorrect,
            });
        }

        let stream_name = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        let stream_type = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        let mut event_messages = EventMessages::new(AsyncBytesWriter::new(self.io.clone()));
        event_messages.stream_begin(stream_id.clone()).await?;

        let mut netstream = NetStream::new(BytesWriter::new());
        let data = netstream.on_status(
            transaction_id,
            &"status".to_string(),
            &"NetStream.Publish.Start".to_string(),
            &"".to_string(),
        )?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }
}
