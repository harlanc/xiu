use super::errors::ClientError;

use crate::chunk::unpacketizer::ChunkUnpacketizer;
use crate::chunk::unpacketizer::UnpackResult;
use crate::chunk::{packetizer::ChunkPacketizer, ChunkInfo};
use crate::{
    chunk::define::{chunk_type, csid_type, CHUNK_SIZE},
    errors::ClientErrorValue,
};

use crate::handshake::handshake::SimpleHandshakeClient;

use crate::messages::define::msg_type_id;
use crate::messages::define::MessageTypes;
use crate::messages::parser::MessageParser;

use crate::amf0::Amf0ValueType;

use liverust_lib::netio::bytes_writer::AsyncBytesWriter;
use liverust_lib::netio::bytes_writer::BytesWriter;
use liverust_lib::netio::netio::NetworkIO;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::handshake::handshake::ClientHandshakeState;
use crate::netconnection::commands::ConnectProperties;
use crate::netconnection::commands::NetConnection;
use crate::netstream::commands::NetStream;
use crate::protocol_control_messages::control_messages::ControlMessages;
use crate::user_control_messages::errors::EventMessagesError;
use crate::user_control_messages::event_messages::EventMessages;

use std::collections::HashMap;

use super::define;
use tokio::{prelude::*, stream::StreamExt, time::timeout};
use tokio_util::codec::{BytesCodec, Framed};

use bytes::BytesMut;

enum ClientSessionState {
    Handshake,
    Connect,
    CreateStream,
    Play,
    PublishingContent,
}

enum ClientSessionPlayState {
    Handshake,
    Connect,
    CreateStream,
    Play,
}

enum ClientSessionPublishState {
    Handshake,
    Connect,
    CreateStream,
    PublishingContent,
}

enum ClientType {
    Play,
    Publish,
}
pub struct ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    packetizer: ChunkPacketizer<S>,
    unpacketizer: ChunkUnpacketizer,
    handshaker: SimpleHandshakeClient<S>,
    io: Rc<RefCell<NetworkIO<S>>>,

    play_state: ClientSessionPlayState,
    publish_state: ClientSessionPublishState,
    state: ClientSessionState,
    client_type: ClientType,
    transaction_id: f64,
}

impl<S> ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(stream: S, timeout: Duration, client_type: ClientType) -> Self {
        let net_io = Rc::new(RefCell::new(NetworkIO::new(stream, timeout)));
        let bytes_writer = AsyncBytesWriter::new(net_io.clone());

        Self {
            io: net_io.clone(),

            packetizer: ChunkPacketizer::new(bytes_writer),
            unpacketizer: ChunkUnpacketizer::new(),
            handshaker: SimpleHandshakeClient::new(net_io.clone()),

            play_state: ClientSessionPlayState::Handshake,
            publish_state: ClientSessionPublishState::Handshake,
            state: ClientSessionState::Handshake,
            client_type: client_type,
            transaction_id: 0.0,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        loop {
            match self.state {
                ClientSessionState::Handshake => {
                    self.handshake().await?;
                }
                ClientSessionState::Connect => {
                    self.send_connect(&(define::TRANSACTION_ID_CONNECT as f64))?;
                }
                ClientSessionState::CreateStream => {
                    self.send_create_stream(&(define::TRANSACTION_ID_CREATE_STREAM as f64))?;
                }
                ClientSessionState::Play => {
                    let stream_name = String::from("stream_name");
                    self.send_play(&0.0, &stream_name, &0.0, &0.0, &false)?;
                }
                ClientSessionState::PublishingContent => {
                    let stream_name = String::from("stream_name");
                    self.send_publish(&0.0, &stream_name, &"live".to_string())?;
                }
            }

            let data = self.io.borrow_mut().read().await?;
            self.unpacketizer.extend_data(&data[..]);
            let result = self.unpacketizer.read_chunk()?;

            match result {
                UnpackResult::ChunkInfo(chunk_info) => {
                    let mut message_parser = MessageParser::new(chunk_info);
                    let mut msg = message_parser.parse()?;

                    self.process_messages(&mut msg)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handshake(&mut self) -> Result<(), ClientError> {
        loop {
            self.handshaker.handshake().await?;
            if self.handshaker.state == ClientHandshakeState::Finish {
                break;
            }

            let data = self.io.borrow_mut().read().await?;
            self.handshaker.extend_data(&data[..]);
        }
        self.state = ClientSessionState::Connect;

        Ok(())
    }

    pub fn process_messages(&mut self, msg: &mut MessageTypes) -> Result<(), ClientError> {
        match msg {
            MessageTypes::Amf0Command {
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
            MessageTypes::SetPeerBandwidth { properties } => self.on_set_peer_bandwidth()?,
            MessageTypes::SetChunkSize { chunk_size } => self
                .unpacketizer
                .update_max_chunk_size(chunk_size.clone() as usize),

            _ => {}
        }
        Ok(())
    }

    pub fn process_amf0_command_message(
        &mut self,
        stream_id: &u32,
        command_name: &Amf0ValueType,
        transaction_id: &Amf0ValueType,
        command_object: &Amf0ValueType,
        others: &mut Vec<Amf0ValueType>,
    ) -> Result<(), ClientError> {
        let empty_cmd_name = &String::new();
        let cmd_name = match command_name {
            Amf0ValueType::UTF8String(str) => str,
            _ => empty_cmd_name,
        };

        let transaction_id = match transaction_id {
            Amf0ValueType::Number(number) => number.clone() as u8,
            _ => 0,
        };

        let empty_cmd_obj: HashMap<String, Amf0ValueType> = HashMap::new();
        let obj = match command_object {
            Amf0ValueType::Object(obj) => obj,
            // Amf0ValueType::Null =>
            _ => &empty_cmd_obj,
        };

        match cmd_name.as_str() {
            "_reslut" => match transaction_id {
                define::TRANSACTION_ID_CONNECT => {
                    self.on_result_connect()?;
                }
                define::TRANSACTION_ID_CREATE_STREAM => {
                    self.on_result_create_stream()?;
                }
                _ => {}
            },
            "_error" => {
                self.on_error()?;
            }
            "onStatus" => {
                match others.remove(0) {
                    Amf0ValueType::Object(obj) => self.on_status(&obj),
                    _ => Err(ClientError {
                        value: ClientErrorValue::Amf0ValueCountNotCorrect,
                    }),
                };
            }

            _ => {}
        }

        Ok(())
    }

    pub fn send_connect(&mut self, transaction_id: &f64) -> Result<(), ClientError> {
        let app_name = String::from("app");
        let properties = ConnectProperties::new(app_name);

        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection.connect(transaction_id, &properties)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;
        Ok(())
    }

    pub fn send_create_stream(&mut self, transaction_id: &f64) -> Result<(), ClientError> {
        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection.create_stream(transaction_id)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;

        Ok(())
    }

    pub fn send_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), ClientError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        let data = netstream.delete_stream(transaction_id, stream_id)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;
        Ok(())
    }

    pub fn send_publish(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        stream_type: &String,
    ) -> Result<(), ClientError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        let data = netstream.publish(transaction_id, stream_name, stream_type)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;

        Ok(())
    }

    pub fn send_play(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        start: &f64,
        duration: &f64,
        reset: &bool,
    ) -> Result<(), ClientError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        let data = netstream.play(transaction_id, stream_name, start, duration, reset)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;

        Ok(())
    }

    pub fn send_set_chunk_size(&mut self) -> Result<(), ClientError> {
        let mut controlmessage = ControlMessages::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_set_chunk_size(CHUNK_SIZE)?;
        Ok(())
    }

    pub fn send_window_acknowledgement_size(
        &mut self,
        window_size: u32,
    ) -> Result<(), ClientError> {
        let mut controlmessage = ControlMessages::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_window_acknowledgement_size(window_size)?;
        Ok(())
    }

    pub fn send_set_buffer_length(&mut self, stream_id: u32, ms: u32) -> Result<(), ClientError> {
        let mut eventmessages = EventMessages::new(AsyncBytesWriter::new(self.io.clone()));
        eventmessages.set_buffer_length(stream_id, ms)?;

        Ok(())
    }

    pub fn on_result_connect(&mut self) -> Result<(), ClientError> {
        self.state = ClientSessionState::CreateStream;
        Ok(())
    }

    pub fn on_result_create_stream(&mut self) -> Result<(), ClientError> {
        match self.client_type {
            ClientType::Play => {
                self.state = ClientSessionState::Play;
            }
            ClientType::Publish => {
                self.state = ClientSessionState::PublishingContent;
            }
        }
        Ok(())
    }

    pub fn on_set_chunk_size(&mut self) -> Result<(), ClientError> {
        Ok(())
    }

    pub fn on_set_peer_bandwidth(&mut self) -> Result<(), ClientError> {
        self.send_window_acknowledgement_size(250000)?;
        Ok(())
    }
    pub fn on_error(&mut self) -> Result<(), ClientError> {
        Ok(())
    }

    pub fn on_status(&mut self, obj: &HashMap<String, Amf0ValueType>) -> Result<(), ClientError> {
        Ok(())
    }
}
