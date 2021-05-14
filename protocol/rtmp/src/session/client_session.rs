use {
    super::{
        common::Common,
        define,
        define::SessionType,
        errors::{SessionError, SessionErrorValue},
    },
    crate::utils::print::print,
    crate::{
        amf0::Amf0ValueType,
        channels::define::ChannelEventProducer,
        chunk::{
            define::{chunk_type, csid_type, CHUNK_SIZE},
            packetizer::ChunkPacketizer,
            unpacketizer::{ChunkUnpacketizer, UnpackResult},
            ChunkInfo,
        },
        handshake::handshake::{ClientHandshakeState, SimpleHandshakeClient},
        messages::{
            define::{msg_type_id, RtmpMessageData},
            parser::MessageParser,
        },
        netconnection::commands::{ConnectProperties, NetConnection},
        netstream::writer::NetStreamWriter,
        protocol_control_messages::writer::ProtocolControlMessagesWriter,
        user_control_messages::writer::EventMessagesWriter,
    },
    bytes::BytesMut,
    networkio::{
        bytes_writer::{AsyncBytesWriter, BytesWriter},
        networkio::NetworkIO,
    },
    std::{collections::HashMap, sync::Arc},
    tokio::{net::TcpStream, sync::Mutex},
};

#[allow(dead_code)]
enum ClientSessionState {
    Handshake,
    Connect,
    CreateStream,
    Play,
    PublishingContent,
    StartPublish,
    WaitStateChange,
}

#[allow(dead_code)]
enum ClientSessionPlayState {
    Handshake,
    Connect,
    CreateStream,
    Play,
}

#[allow(dead_code)]
enum ClientSessionPublishState {
    Handshake,
    Connect,
    CreateStream,
    PublishingContent,
}
#[allow(dead_code)]
pub enum ClientType {
    Play,
    Publish,
}
pub struct ClientSession {
    io: Arc<Mutex<NetworkIO>>,
    common: Common,

    handshaker: SimpleHandshakeClient,

    packetizer: ChunkPacketizer,
    unpacketizer: ChunkUnpacketizer,

    app_name: String,
    stream_name: String,
    session_type: u8,
    session_id: u64,

    state: ClientSessionState,
    client_type: ClientType,
    connect_command_object: Option<HashMap<String, Amf0ValueType>>,
}

impl ClientSession {
    #[allow(dead_code)]
    pub fn new(
        stream: TcpStream,
        client_type: ClientType,
        app_name: String,
        stream_name: String,
        event_producer: ChannelEventProducer,
        session_id: u64,
    ) -> Self {
        let net_io = Arc::new(Mutex::new(NetworkIO::new(stream)));

        Self {
            io: Arc::clone(&net_io),
            common: Common::new(Arc::clone(&net_io), event_producer, SessionType::Client),

            handshaker: SimpleHandshakeClient::new(Arc::clone(&net_io)),

            packetizer: ChunkPacketizer::new(Arc::clone(&net_io)),
            unpacketizer: ChunkUnpacketizer::new(),

            app_name: app_name,
            stream_name: stream_name,
            client_type: client_type,

            state: ClientSessionState::Handshake,
            session_type: 0,
            session_id: session_id,
            connect_command_object: None,
        }
    }

    pub fn set_connect_command_object(&mut self, object: HashMap<String, Amf0ValueType>) {
        self.connect_command_object = Some(object);
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            match self.state {
                ClientSessionState::Handshake => {
                    println!("handshake");
                    self.handshake().await?;
                    continue;
                }
                ClientSessionState::Connect => {
                    println!("connect");
                    self.send_connect(&(define::TRANSACTION_ID_CONNECT as f64))
                        .await?;
                    self.state = ClientSessionState::WaitStateChange;
                }
                ClientSessionState::CreateStream => {
                    println!("CreateStream");
                    self.send_create_stream(&(define::TRANSACTION_ID_CREATE_STREAM as f64))
                        .await?;
                    self.state = ClientSessionState::WaitStateChange;
                }
                ClientSessionState::Play => {
                    self.send_play(&0.0, &self.stream_name.clone(), &0.0, &0.0, &false)
                        .await?;
                    self.state = ClientSessionState::WaitStateChange;
                }
                ClientSessionState::PublishingContent => {
                    println!("PublishingContent");
                    self.send_publish(&0.0, &self.stream_name.clone(), &"live".to_string())
                        .await?;
                    self.state = ClientSessionState::WaitStateChange;
                }
                ClientSessionState::StartPublish => {
                    println!("StartPublish");
                    self.common.send_channel_data().await?;
                }
                ClientSessionState::WaitStateChange => {}
            }

            let data = self.io.lock().await.read().await?;
            self.unpacketizer.extend_data(&data[..]);
            let result = self.unpacketizer.read_chunk()?;

            match result {
                UnpackResult::ChunkInfo(chunk_info) => {
                    let mut message_parser =
                        MessageParser::new(chunk_info.clone(), self.session_type);
                    let mut msg = message_parser.parse()?;
                    let timestamp = chunk_info.message_header.timestamp;

                    self.process_messages(&mut msg, &timestamp).await?;
                }
                _ => {}
            }
        }

        // Ok(())
    }

    async fn handshake(&mut self) -> Result<(), SessionError> {
        loop {
            self.handshaker.handshake().await?;
            if self.handshaker.state == ClientHandshakeState::Finish {
                println!("handshake finish");
                break;
            }

            let data = self.io.lock().await.read().await?;
            print(data.clone());
            self.handshaker.extend_data(&data[..]);
        }

        self.state = ClientSessionState::Connect;

        Ok(())
    }

    pub async fn process_messages(
        &mut self,
        msg: &mut RtmpMessageData,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        match msg {
            RtmpMessageData::Amf0Command {
                command_name,
                transaction_id,
                command_object,
                others,
            } => {
                self.on_amf0_command_message(command_name, transaction_id, command_object, others)
                    .await?
            }
            RtmpMessageData::SetPeerBandwidth { properties } => {
                print!("{}", properties.window_size);
                self.on_set_peer_bandwidth().await?
            }
            RtmpMessageData::SetChunkSize { chunk_size } => self.on_set_chunk_size(chunk_size)?,

            RtmpMessageData::StreamBegin { stream_id } => self.on_stream_begin(stream_id)?,

            RtmpMessageData::StreamIsRecorded { stream_id } => {
                self.on_stream_is_recorded(stream_id)?
            }

            RtmpMessageData::AudioData { data } => self.common.on_audio_data(data, timestamp)?,

            RtmpMessageData::VideoData { data } => self.common.on_video_data(data, timestamp)?,

            _ => {}
        }
        Ok(())
    }

    pub async fn on_amf0_command_message(
        &mut self,
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
            Amf0ValueType::Number(number) => number.clone() as u8,
            _ => 0,
        };

        let empty_cmd_obj: HashMap<String, Amf0ValueType> = HashMap::new();
        let _ = match command_object {
            Amf0ValueType::Object(obj) => obj,
            // Amf0ValueType::Null =>
            _ => &empty_cmd_obj,
        };

        match cmd_name.as_str() {
            "_result" => match transaction_id {
                define::TRANSACTION_ID_CONNECT => {
                    self.on_result_connect().await?;
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
                    Amf0ValueType::Object(obj) => self.on_status(&obj).await?,
                    _ => {
                        return Err(SessionError {
                            value: SessionErrorValue::Amf0ValueCountNotCorrect,
                        })
                    }
                };
            }

            _ => {}
        }

        Ok(())
    }

    pub async fn send_connect(&mut self, transaction_id: &f64) -> Result<(), SessionError> {
        self.send_set_chunk_size().await?;

        // let properties = ConnectProperties::new(self.app_name.clone());
        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection
            .connect_with_value(transaction_id, self.connect_command_object.clone().unwrap())?;

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

    pub async fn send_create_stream(&mut self, transaction_id: &f64) -> Result<(), SessionError> {
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

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub async fn send_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), SessionError> {
        let mut netstream = NetStreamWriter::new(BytesWriter::new(), Arc::clone(&self.io));
        netstream.delete_stream(transaction_id, stream_id).await?;

        Ok(())
    }

    pub async fn send_publish(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        stream_type: &String,
    ) -> Result<(), SessionError> {
        let mut netstream = NetStreamWriter::new(BytesWriter::new(), Arc::clone(&self.io));
        netstream
            .publish(transaction_id, stream_name, stream_type)
            .await?;

        Ok(())
    }

    pub async fn send_play(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        start: &f64,
        duration: &f64,
        reset: &bool,
    ) -> Result<(), SessionError> {
        let mut netstream = NetStreamWriter::new(BytesWriter::new(), Arc::clone(&self.io));
        netstream
            .play(transaction_id, stream_name, start, duration, reset)
            .await?;

        Ok(())
    }

    pub async fn send_set_chunk_size(&mut self) -> Result<(), SessionError> {
        let mut controlmessage =
            ProtocolControlMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_set_chunk_size(CHUNK_SIZE).await?;
        Ok(())
    }

    pub async fn send_window_acknowledgement_size(
        &mut self,
        window_size: u32,
    ) -> Result<(), SessionError> {
        let mut controlmessage =
            ProtocolControlMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage
            .write_window_acknowledgement_size(window_size)
            .await?;
        Ok(())
    }

    pub async fn send_set_buffer_length(
        &mut self,
        stream_id: u32,
        ms: u32,
    ) -> Result<(), SessionError> {
        let mut eventmessages = EventMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        eventmessages.write_set_buffer_length(stream_id, ms).await?;

        Ok(())
    }

    pub async fn on_result_connect(&mut self) -> Result<(), SessionError> {
        let mut controlmessage =
            ProtocolControlMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_acknowledgement(3107).await?;

        let mut netstream = NetStreamWriter::new(BytesWriter::new(), Arc::clone(&self.io));
        netstream
            .release_stream(&(define::TRANSACTION_ID_CONNECT as f64), &self.stream_name)
            .await?;
        netstream
            .fcpublish(&(define::TRANSACTION_ID_CONNECT as f64), &self.stream_name)
            .await?;

        self.state = ClientSessionState::CreateStream;

        Ok(())
    }

    pub fn on_result_create_stream(&mut self) -> Result<(), SessionError> {
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

    pub fn on_set_chunk_size(&mut self, chunk_size: &mut u32) -> Result<(), SessionError> {
        self.unpacketizer
            .update_max_chunk_size(chunk_size.clone() as usize);
        Ok(())
    }

    pub fn on_stream_is_recorded(&mut self, stream_id: &mut u32) -> Result<(), SessionError> {
        Ok(())
    }

    pub fn on_stream_begin(&mut self, stream_id: &mut u32) -> Result<(), SessionError> {
        Ok(())
    }

    pub async fn on_set_peer_bandwidth(&mut self) -> Result<(), SessionError> {
        self.send_window_acknowledgement_size(250000).await?;
        Ok(())
    }

    pub fn on_error(&mut self) -> Result<(), SessionError> {
        Ok(())
    }

    pub async fn on_status(
        &mut self,
        obj: &HashMap<String, Amf0ValueType>,
    ) -> Result<(), SessionError> {
        println!("on_status===");
        if let Some(Amf0ValueType::UTF8String(code_info)) = obj.get("code") {
            match &code_info[..] {
                "NetStream.Publish.Start" => {
                    self.state = ClientSessionState::StartPublish;
                    self.common
                        .subscribe_from_channels(
                            self.app_name.clone(),
                            self.stream_name.clone(),
                            self.session_id,
                        )
                        .await?;
                }
                "NetStream.Publish.Reset" => {}

                // "NetStream.Play.Start" => self.common.publish_to_channels(
                //     self.app_name.clone(),
                //     self.stream_name.clone(),
                //     connect_command_object,
                // ),

                _ => {}
            }
        }
        println!("{}", obj.len());
        Ok(())
    }
}
