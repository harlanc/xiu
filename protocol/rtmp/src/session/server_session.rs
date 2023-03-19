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
        config, handshake,
        handshake::{define::ServerHandshakeState, handshake_server::HandshakeServer},
        messages::{define::RtmpMessageData, parser::MessageParser},
        netconnection::writer::{ConnectProperties, NetConnection},
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
    DeleteStream,
    Play,
}

pub struct ServerSession {
    pub app_name: String,
    pub stream_name: String,
    pub url_parameters: String,
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
    pub session_id: Uuid,
    connect_properties: ConnectProperties,
}

impl ServerSession {
    pub fn new(stream: TcpStream, event_producer: ChannelEventProducer) -> Self {
        let remote_addr = if let Ok(addr) = stream.peer_addr() {
            log::info!("server session: {}", addr.to_string());
            Some(addr)
        } else {
            None
        };

        let net_io = Arc::new(Mutex::new(BytesIO::new(stream)));
        let subscriber_id = Uuid::new_v4();
        Self {
            app_name: String::from(""),
            stream_name: String::from(""),
            url_parameters: String::from(""),
            io: Arc::clone(&net_io),
            handshaker: HandshakeServer::new(Arc::clone(&net_io)),
            unpacketizer: ChunkUnpacketizer::new(),
            state: ServerSessionState::Handshake,
            common: Common::new(
                Arc::clone(&net_io),
                event_producer,
                SessionType::Server,
                remote_addr,
            ),
            session_id: subscriber_id,
            bytesio_data: BytesMut::new(),
            has_remaing_data: false,
            connect_properties: ConnectProperties::default(),
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
                ServerSessionState::DeleteStream => {
                    return Ok(());
                }
            }
        }

        //Ok(())
    }

    async fn handshake(&mut self) -> Result<(), SessionError> {
        let mut bytes_len = 0;

        while bytes_len < handshake::define::RTMP_HANDSHAKE_SIZE {
            self.bytesio_data = self.io.lock().await.read().await?;
            bytes_len += self.bytesio_data.len();
            self.handshaker.extend_data(&self.bytesio_data[..]);
        }

        self.handshaker.handshake().await?;

        if let ServerHandshakeState::Finish = self.handshaker.state() {
            self.state = ServerSessionState::ReadChunk;
            let left_bytes = self.handshaker.get_remaining_bytes();
            if !left_bytes.is_empty() {
                self.unpacketizer.extend_data(&left_bytes[..]);
                self.has_remaing_data = true;
            }
            log::info!("[ S->C ] [send_set_chunk_size] ");
            self.send_set_chunk_size().await?;
            return Ok(());
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
                        .unpublish_to_channels(
                            self.app_name.clone(),
                            self.stream_name.clone(),
                            self.session_id,
                        )
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
                if let UnpackResult::Chunks(chunks) = rv {
                    for chunk_info in chunks {
                        let timestamp = chunk_info.message_header.timestamp;
                        let msg_stream_id = chunk_info.message_header.msg_streamd_id;

                        let mut msg = MessageParser::new(chunk_info).parse()?;
                        self.process_messages(&mut msg, &msg_stream_id, &timestamp)
                            .await?;
                    }
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
                        self.session_id,
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
                self.on_set_chunk_size(*chunk_size as usize)?;
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
                self.on_connect(transaction_id, obj).await?;
            }
            "createStream" => {
                log::info!("[ S<-C ] [create stream] ");
                self.on_create_stream(transaction_id).await?;
            }
            "deleteStream" => {
                if !others.is_empty() {
                    let stream_id = match others.pop() {
                        Some(Amf0ValueType::Number(streamid)) => streamid,
                        _ => 0.0,
                    };

                    log::info!(
                        "[ S<-C ] [delete stream] app_name: {}, stream_name: {}",
                        self.app_name,
                        self.stream_name
                    );

                    self.on_delete_stream(transaction_id, &stream_id).await?;
                    self.state = ServerSessionState::DeleteStream;
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
        log::info!(
            "[ S<-C ] [set chunk size]  app_name: {}, stream_name: {}, chunk size: {}",
            self.app_name,
            self.stream_name,
            chunk_size
        );
        self.unpacketizer.update_max_chunk_size(chunk_size);
        Ok(())
    }

    fn parse_connect_properties(&mut self, command_obj: &HashMap<String, Amf0ValueType>) {
        for (property, value) in command_obj {
            match property.as_str() {
                "app" => {
                    if let Amf0ValueType::UTF8String(app) = value {
                        self.connect_properties.app = Some(app.clone());
                    }
                }
                "flashVer" => {
                    if let Amf0ValueType::UTF8String(flash_ver) = value {
                        self.connect_properties.flash_ver = Some(flash_ver.clone());
                    }
                }
                "swfUrl" => {
                    if let Amf0ValueType::UTF8String(swf_url) = value {
                        self.connect_properties.swf_url = Some(swf_url.clone());
                    }
                }
                "tcUrl" => {
                    if let Amf0ValueType::UTF8String(tc_url) = value {
                        self.connect_properties.tc_url = Some(tc_url.clone());
                    }
                }
                "fpad" => {
                    if let Amf0ValueType::Boolean(fpad) = value {
                        self.connect_properties.fpad = Some(*fpad);
                    }
                }
                "audioCodecs" => {
                    if let Amf0ValueType::Number(audio_codecs) = value {
                        self.connect_properties.audio_codecs = Some(*audio_codecs);
                    }
                }
                "videoCodecs" => {
                    if let Amf0ValueType::Number(video_codecs) = value {
                        self.connect_properties.video_codecs = Some(*video_codecs);
                    }
                }
                "videoFunction" => {
                    if let Amf0ValueType::Number(video_function) = value {
                        self.connect_properties.video_function = Some(*video_function);
                    }
                }
                "pageUrl" => {
                    if let Amf0ValueType::UTF8String(page_url) = value {
                        self.connect_properties.page_url = Some(page_url.clone());
                    }
                }
                "objectEncoding" => {
                    if let Amf0ValueType::Number(object_encoding) = value {
                        self.connect_properties.object_encoding = Some(*object_encoding);
                    }
                }
                _ => {
                    log::warn!("unknown connect properties: {}:{:?}", property, value);
                }
            }
        }
    }

    async fn on_connect(
        &mut self,
        transaction_id: &f64,
        command_obj: &HashMap<String, Amf0ValueType>,
    ) -> Result<(), SessionError> {
        self.parse_connect_properties(command_obj);
        log::info!("connect properties: {:?}", self.connect_properties);
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
                transaction_id,
                define::FMSVER,
                &define::CAPABILITIES,
                &String::from("NetConnection.Connect.Success"),
                define::LEVEL,
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
            .unpublish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.session_id,
            )
            .await?;

        let mut netstream = NetStreamWriter::new(Arc::clone(&self.io));
        netstream
            .write_on_status(
                transaction_id,
                "status",
                "NetStream.DeleteStream.Suceess",
                "",
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

    fn get_request_url(&mut self, raw_stream_name: String) -> String {
        if let Some(tc_url) = &self.connect_properties.tc_url {
            format!("{}/{}", tc_url, raw_stream_name)
        } else {
            format!("{}/{}", self.app_name.clone(), raw_stream_name)
        }
    }
    /*parse the raw stream name to get real stream name and the URL parameters*/
    fn parse_raw_stream_name(&mut self, raw_stream_name: String) {
        let data: Vec<&str> = raw_stream_name.split('?').collect();
        self.stream_name = data[0].to_string();
        if data.len() > 1 {
            self.url_parameters = data[1].to_string();
        }
    }

    #[allow(clippy::never_loop)]
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
            index += 1;
            stream_name = match other_values.remove(0) {
                Amf0ValueType::UTF8String(val) => Some(val),
                _ => None,
            };

            if index >= length {
                break;
            }
            index += 1;
            start = match other_values.remove(0) {
                Amf0ValueType::Number(val) => Some(val),
                _ => None,
            };

            if index >= length {
                break;
            }
            index += 1;
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
        event_messages.write_stream_begin(*stream_id).await?;
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
            .write_on_status(transaction_id, "status", "NetStream.Play.Reset", "reset")
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                "status",
                "NetStream.Play.Start",
                "play start",
            )
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                "status",
                "NetStream.Data.Start",
                "data start.",
            )
            .await?;

        netstream
            .write_on_status(
                transaction_id,
                "status",
                "NetStream.Play.PublishNotify",
                "play publish notify.",
            )
            .await?;

        event_messages.write_stream_is_record(*stream_id).await?;

        let raw_stream_name = stream_name.unwrap();
        self.parse_raw_stream_name(raw_stream_name.clone());

        log::info!(
            "[ S->C ] [stream is record]  app_name: {}, stream_name: {}, url parameters: {}",
            self.app_name,
            self.stream_name,
            self.url_parameters
        );

        /*Now it can update the request url*/
        self.common.request_url = self.get_request_url(raw_stream_name);
        self.common
            .subscribe_from_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.session_id,
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

        let raw_stream_name = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        self.parse_raw_stream_name(raw_stream_name.clone());
        /*Now it can update the request url*/
        self.common.request_url = self.get_request_url(raw_stream_name);

        let _ = match other_values.remove(0) {
            Amf0ValueType::UTF8String(val) => val,
            _ => {
                return Err(SessionError {
                    value: SessionErrorValue::Amf0ValueCountNotCorrect,
                });
            }
        };

        log::info!(
            "[ S<-C ] [publish]  app_name: {}, stream_name: {}, url parameters: {}",
            self.app_name,
            self.stream_name,
            self.url_parameters
        );

        log::info!(
            "[ S->C ] [stream begin]  app_name: {}, stream_name: {}, url parameters: {}",
            self.app_name,
            self.stream_name,
            self.url_parameters
        );

        let mut event_messages = EventMessagesWriter::new(AsyncBytesWriter::new(self.io.clone()));
        event_messages.write_stream_begin(*stream_id).await?;

        let mut netstream = NetStreamWriter::new(Arc::clone(&self.io));
        netstream
            .write_on_status(transaction_id, "status", "NetStream.Publish.Start", "")
            .await?;
        log::info!(
            "[ S->C ] [NetStream.Publish.Start]  app_name: {}, stream_name: {}",
            self.app_name,
            self.stream_name
        );

        self.common
            .publish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.session_id,
            )
            .await?;

        Ok(())
    }
}
