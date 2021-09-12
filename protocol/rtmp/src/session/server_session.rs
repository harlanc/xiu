use {
    super::{
        common::Common,
        define,
        define::SessionType,
        errors::{SessionError, SessionErrorValue},
    },
    crate::{
        amf0::Amf0ValueType,
        channels::define::ChannelEventProducer,
        chunk::{
            define::CHUNK_SIZE,
            unpacketizer::{ChunkUnpacketizer, UnpackResult},
        },
        config,
        handshake::{define::ServerHandshakeState, handshake_server::HandshakeServer},
        messages::{define::RtmpMessageData, parser::MessageParser},
        netconnection::writer::NetConnection,
        netstream::writer::NetStreamWriter,
        protocol_control_messages::writer::ProtocolControlMessagesWriter,
        user_control_messages::writer::EventMessagesWriter,
    },
    bytes::BytesMut,
    bytesio::{bytes_writer::AsyncBytesWriter, bytesio::BytesIO},
    std::{collections::HashMap, sync::Arc, time::Duration},
    tokio::{net::TcpStream, sync::Mutex},
    uuid::Uuid,
};

enum ServerSessionState {
    Handshake,
    ReadChunk,
    // OnConnect,
    // OnCreateStream,
    //Publish,
    Play,
}

pub struct ServerSession {
    pub app_name: String,
    pub stream_name: String,

    io: Arc<Mutex<BytesIO>>,
    handshaker: HandshakeServer,

    unpacketizer: ChunkUnpacketizer,

    state: ServerSessionState,

    pub common: Common,

    bytesio_data: BytesMut,
    has_remaing_data: bool,

    /* Used to mark the subscriber's the data producer
    in channels and delete it from map when unsubscribe
    is called. */
    pub subscriber_id: Uuid,

    connect_command_object: Option<HashMap<String, Amf0ValueType>>,
}

impl ServerSession {
    pub fn new(stream: TcpStream, event_producer: ChannelEventProducer) -> Self {
        let net_io = Arc::new(Mutex::new(BytesIO::new(stream)));
        let subscriber_id = Uuid::new_v4();
        Self {
            app_name: String::from(""),
            stream_name: String::from(""),

            io: Arc::clone(&net_io),
            handshaker: HandshakeServer::new(Arc::clone(&net_io)),

            unpacketizer: ChunkUnpacketizer::new(),

            state: ServerSessionState::Handshake,

            common: Common::new(Arc::clone(&net_io), event_producer, SessionType::Server),

            subscriber_id,
            bytesio_data: BytesMut::new(),
            has_remaing_data: false,

            connect_command_object: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            match self.state {
                ServerSessionState::Handshake => {
                    self.handshake().await?;
                }
                ServerSessionState::ReadChunk => {
                    self.read_parse_chunks().await?;
                }
                ServerSessionState::Play => {
                    self.play().await?;
                }
            }
        }

        //Ok(())
    }

    async fn handshake(&mut self) -> Result<(), SessionError> {
        self.bytesio_data = self.io.lock().await.read().await?;

        self.handshaker.extend_data(&self.bytesio_data[..]);
        self.handshaker.handshake().await?;

        match self.handshaker.state() {
            ServerHandshakeState::Finish => {
                self.state = ServerSessionState::ReadChunk;

                let left_bytes = self.handshaker.get_remaining_bytes();
                if left_bytes.len() > 0 {
                    self.unpacketizer.extend_data(&left_bytes[..]);
                    self.has_remaing_data = true;
                }
                log::info!("[ S->C ] [send_set_chunk_size] ");
                self.send_set_chunk_size().await?;
                return Ok(());
            }
            _ => {}
        }

        Ok(())
    }

    async fn read_parse_chunks(&mut self) -> Result<(), SessionError> {
        if !self.has_remaing_data {
            match self
                .io
                .lock()
                .await
                .read_timeout(Duration::from_secs(2))
                .await
            {
                Ok(data) => {
                    self.bytesio_data = data;
                }
                Err(err) => {
                    self.common
                        .unpublish_to_channels(self.app_name.clone(), self.stream_name.clone())
                        .await?;

                    return Err(SessionError {
                        value: SessionErrorValue::BytesIOError(err),
                    });
                }
            }

            self.unpacketizer.extend_data(&self.bytesio_data[..]);
        }

        self.has_remaing_data = false;

        loop {
            let result = self.unpacketizer.read_chunks();

            if let Ok(rv) = result {
                match rv {
                    UnpackResult::Chunks(chunks) => {
                        for chunk_info in chunks {
                            let timestamp = chunk_info.message_header.timestamp;
                            let msg_stream_id = chunk_info.message_header.msg_streamd_id;

                            let mut msg = MessageParser::new(chunk_info).parse()?;
                            self.process_messages(&mut msg, &msg_stream_id, &timestamp)
                                .await?;
                        }
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn play(&mut self) -> Result<(), SessionError> {
        match self.common.send_channel_data().await {
            Ok(_) => {}

            Err(err) => {
                self.common
                    .unsubscribe_from_channels(
                        self.app_name.clone(),
                        self.stream_name.clone(),
                        self.subscriber_id,
                    )
                    .await?;
                return Err(err);
            }
        }

        Ok(())
    }

    pub async fn send_set_chunk_size(&mut self) -> Result<(), SessionError> {
        let mut controlmessage =
            ProtocolControlMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_set_chunk_size(CHUNK_SIZE).await?;

        Ok(())
    }

    pub async fn process_messages(
        &mut self,
        rtmp_msg: &mut RtmpMessageData,
        msg_stream_id: &u32,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        match rtmp_msg {
            RtmpMessageData::Amf0Command {
                command_name,
                transaction_id,
                command_object,
                others,
            } => {
                self.on_amf0_command_message(
                    msg_stream_id,
                    command_name,
                    transaction_id,
                    command_object,
                    others,
                )
                .await?
            }
            RtmpMessageData::SetChunkSize { chunk_size } => {
                self.on_set_chunk_size(chunk_size.clone() as usize)?;
            }
            RtmpMessageData::AudioData { data } => {
                self.common.on_audio_data(data, timestamp)?;
            }
            RtmpMessageData::VideoData { data } => {
                self.common.on_video_data(data, timestamp)?;
            }
            RtmpMessageData::AmfData { raw_data } => {
                self.common.on_meta_data(raw_data, timestamp)?;
            }

            _ => {}
        }
        Ok(())
    }

    pub async fn on_amf0_command_message(
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
                log::info!("[ S<-C ] [connect] ");
                self.on_connect(&transaction_id, &obj).await?;
            }
            "createStream" => {
                log::info!("[ S<-C ] [create stream] ");
                self.on_create_stream(transaction_id).await?;
            }
            "deleteStream" => {
                if others.len() > 0 {
                    let stream_id = match others.pop() {
                        Some(val) => match val {
                            Amf0ValueType::Number(streamid) => streamid,
                            _ => 0.0,
                        },
                        _ => 0.0,
                    };
                    log::info!(
                        "[ S<-C ] [delete stream] app_name: {}, stream_name: {}",
                        self.app_name,
                        self.stream_name
                    );

                    self.on_delete_stream(transaction_id, &stream_id).await?;
                }
            }
            "play" => {
                log::info!(
                    "[ S<-C ] [play]  app_name: {}, stream_name: {}",
                    self.app_name,
                    self.stream_name
                );
                self.unpacketizer.session_type = config::SERVER_PULL;
                self.on_play(transaction_id, stream_id, others).await?;
            }
            "publish" => {
                self.unpacketizer.session_type = config::SERVER_PUSH;
                self.on_publish(transaction_id, stream_id, others).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn on_set_chunk_size(&mut self, chunk_size: usize) -> Result<(), SessionError> {
        self.unpacketizer.update_max_chunk_size(chunk_size);
        Ok(())
    }

    async fn on_connect(
        &mut self,
        transaction_id: &f64,
        command_obj: &HashMap<String, Amf0ValueType>,
    ) -> Result<(), SessionError> {
        self.connect_command_object = Some(command_obj.clone());
        let mut control_message =
            ProtocolControlMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        log::info!("[ S->C ] [set window_acknowledgement_size]");
        control_message
            .write_window_acknowledgement_size(define::WINDOW_ACKNOWLEDGEMENT_SIZE)
            .await?;

        log::info!("[ S->C ] [set set_peer_bandwidth]",);
        control_message
            .write_set_peer_bandwidth(
                define::PEER_BANDWIDTH,
                define::peer_bandwidth_limit_type::DYNAMIC,
            )
            .await?;

        let obj_encoding = command_obj.get("objectEncoding");
        let encoding = match obj_encoding {
            Some(Amf0ValueType::Number(encoding)) => encoding,
            _ => &define::OBJENCODING_AMF0,
        };

        let app_name = command_obj.get("app");
        self.app_name = match app_name {
            Some(Amf0ValueType::UTF8String(app)) => app.clone(),
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::NoAppName,
                });
            }
        };

        let mut netconnection = NetConnection::new(Arc::clone(&self.io));
        log::info!("[ S->C ] [set connect_response]",);
        netconnection
            .write_connect_response(
                &transaction_id,
                &define::FMSVER.to_string(),
                &define::CAPABILITIES,
                &String::from("NetConnection.Connect.Success"),
                &define::LEVEL.to_string(),
                &String::from("Connection Succeeded."),
                encoding,
            )
            .await?;

        Ok(())
    }

    pub async fn on_create_stream(&mut self, transaction_id: &f64) -> Result<(), SessionError> {
        let mut netconnection = NetConnection::new(Arc::clone(&self.io));
        netconnection
            .write_create_stream_response(transaction_id, &define::STREAM_ID)
            .await?;

        log::info!(
            "[ S->C ] [create_stream_response]  app_name: {}",
            self.app_name,
        );

        Ok(())
    }

    pub async fn on_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), SessionError> {
        self.common
            .unpublish_to_channels(self.app_name.clone(), self.stream_name.clone())
            .await?;

        let mut netstream = NetStreamWriter::new(Arc::clone(&self.io));
        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.DeleteStream.Suceess".to_string(),
                &"".to_string(),
            )
            .await?;

        //self.unsubscribe_from_channels().await?;
        log::info!(
            "[ S->C ] [delete stream success]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );
        log::trace!("{}", stream_id);

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
            //index = index + 1;
            reset = match other_values.remove(0) {
                Amf0ValueType::Boolean(val) => Some(val),
                _ => None,
            };
            break;
        }

        let mut event_messages = EventMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        event_messages.write_stream_begin(stream_id.clone()).await?;
        log::info!(
            "[ S->C ] [stream begin]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );
        log::trace!(
            "{} {} {}",
            start.is_some(),
            duration.is_some(),
            reset.is_some()
        );

        let mut netstream = NetStreamWriter::new(Arc::clone(&self.io));
        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.Play.Reset".to_string(),
                &"reset".to_string(),
            )
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.Play.Start".to_string(),
                &"play start".to_string(),
            )
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.Data.Start".to_string(),
                &"data start.".to_string(),
            )
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.Play.PublishNotify".to_string(),
                &"play publish notify.".to_string(),
            )
            .await?;

        event_messages
            .write_stream_is_record(stream_id.clone())
            .await?;
        log::info!(
            "[ S->C ] [stream is record]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );
        self.stream_name = stream_name.clone().unwrap();
        self.common
            .subscribe_from_channels(
                self.app_name.clone(),
                stream_name.unwrap(),
                self.subscriber_id,
            )
            .await?;

        self.state = ServerSessionState::Play;

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

        self.stream_name = stream_name;

        let _ = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        log::info!(
            "[ S<-C ] [publish]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );

        log::info!(
            "[ S->C ] [stream begin]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );

        let mut event_messages = EventMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        event_messages.write_stream_begin(stream_id.clone()).await?;

        let mut netstream = NetStreamWriter::new(Arc::clone(&self.io));
        netstream
            .write_on_status(
                transaction_id,
                &"status".to_string(),
                &"NetStream.Publish.Start".to_string(),
                &"".to_string(),
            )
            .await?;
        log::info!(
            "[ S->C ] [NetStream.Publish.Start]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );

        self.common
            .publish_to_channels(self.app_name.clone(), self.stream_name.clone())
            .await?;

        Ok(())
    }
}
