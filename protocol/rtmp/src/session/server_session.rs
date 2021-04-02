use super::define;
use super::errors::SessionError;
use super::errors::SessionErrorValue;
use crate::handshake::handshake::{ComplexHandshakeServer, SimpleHandshakeServer};
use crate::{amf0::Amf0ValueType, chunk::unpacketizer::UnpackResult};
use crate::{
    application,
    chunk::{unpacketizer::ChunkUnpacketizer, ChunkInfo},
};
use crate::{channels, chunk::packetizer::ChunkPacketizer};
use crate::{
    chunk::define::CHUNK_SIZE,
    chunk::define::{chunk_type, csid_type},
};

use crate::messages::define::msg_type_id;
use crate::messages::define::RtmpMessageData;

use crate::messages::parser::MessageParser;
use bytes::BytesMut;

use netio::bytes_reader::BytesReader;
use netio::bytes_writer::AsyncBytesWriter;
use netio::bytes_writer::BytesWriter;
use netio::netio::NetworkIO;
use std::{borrow::BorrowMut, time::Duration};

use crate::channels::define::ChannelEvent;
use crate::channels::define::MultiConsumerForData;
use crate::channels::define::MultiProducerForEvent;
use crate::channels::define::SingleProducerForData;
use crate::netconnection::commands::NetConnection;
use crate::netstream::commands::NetStream;
use crate::protocol_control_messages::control_messages::ControlMessages;

use crate::user_control_messages::event_messages::EventMessages;

use std::collections::HashMap;

use tokio::sync::broadcast;
use tokio::{io::ReadHalf, net::TcpStream};

use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::channels::define::ChannelData;
use crate::channels::errors::ChannelError;
use crate::channels::errors::ChannelErrorValue;

use crate::handshake::handshake::ServerHandshakeState;

use crate::utils;
use log::{debug, log, log_enabled, Level};
use std::fmt;

enum ServerSessionState {
    Handshake,
    ReadChunk,
    // OnConnect,
    // OnCreateStream,
    //Publish,
    Play,
}

pub struct ServerSession {
    app_name: String,

    io: Arc<Mutex<NetworkIO>>,
    simple_handshaker: SimpleHandshakeServer,
    complex_handshaker: ComplexHandshakeServer,

    packetizer: ChunkPacketizer,
    unpacketizer: ChunkUnpacketizer,

    state: ServerSessionState,

    event_producer: MultiProducerForEvent,

    //send video, audio or metadata from publish server session to player server sessions
    data_producer: SingleProducerForData,
    //receive video, audio or metadata from publish server session and send out to player
    data_consumer: MultiConsumerForData,

    session_id: u8,
}

impl ServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: MultiProducerForEvent,
        timeout: Duration,
        session_id: u8,
    ) -> Self {
        let net_io = Arc::new(Mutex::new(NetworkIO::new(stream, timeout)));
        //only used for init,since I don't found a better way to deal with this.
        let (init_producer, init_consumer) = broadcast::channel(1);
        // let reader = BytesReader::new(BytesMut::new());

        Self {
            app_name: String::from(""),

            io: Arc::clone(&net_io),
            simple_handshaker: SimpleHandshakeServer::new(Arc::clone(&net_io)),
            complex_handshaker: ComplexHandshakeServer::new(Arc::clone(&net_io)),
            packetizer: ChunkPacketizer::new(Arc::clone(&net_io)),
            unpacketizer: ChunkUnpacketizer::new(),

            state: ServerSessionState::Handshake,

            event_producer,
            data_producer: init_producer,
            data_consumer: init_consumer,
            session_id: session_id,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        let duration = Duration::new(10, 10);

        let mut remaining_bytes: BytesMut = BytesMut::new();

        loop {
            let mut net_io_data: BytesMut = BytesMut::new();
            if remaining_bytes.len() <= 0 {
                net_io_data = self.io.lock().await.read().await?;

                utils::print::print(net_io_data.clone());
            }

            match self.state {
                ServerSessionState::Handshake => {
                    if self.session_id > 0 {
                        // utils::print::printu8(net_io_data.clone());
                    }

                    self.simple_handshaker.extend_data(&net_io_data[..]);
                    self.simple_handshaker.handshake().await?;

                    match self.simple_handshaker.state {
                        ServerHandshakeState::Finish => {
                            self.state = ServerSessionState::ReadChunk;
                            remaining_bytes = self.simple_handshaker.get_remaining_bytes();
                        }
                        _ => continue,
                    }
                }
                ServerSessionState::ReadChunk => {
                    if self.session_id > 0 {
                        // utils::print::printu8(net_io_data.clone());
                    }

                    if remaining_bytes.len() > 0 {
                        let bytes = std::mem::replace(&mut remaining_bytes, BytesMut::new());
                        self.unpacketizer.extend_data(&bytes[..]);
                    } else {
                        self.unpacketizer.extend_data(&net_io_data[..]);
                    }

                    let result = self.unpacketizer.read_chunks();

                    let rv = match result {
                        Ok(val) => val,
                        Err(err) => {
                            // return Err(SessionError {
                            //     value: SessionErrorValue::UnPackError(err),
                            // })
                            continue;
                        }
                    };

                    match rv {
                        UnpackResult::Chunks(chunks) => {
                            for chunk_info in chunks.iter() {
                                let msg_stream_id = chunk_info.message_header.msg_streamd_id;
                                let timestamp = chunk_info.message_header.timestamp;

                                let mut message_parser = MessageParser::new(chunk_info.clone());
                                let mut msg = message_parser.parse()?;

                                self.process_messages(&mut msg, &msg_stream_id, &timestamp)
                                    .await?;
                            }
                        }
                        _ => {}
                    }
                }

                //when in play state, only transfer publisher's video/audio/metadta to player.
                ServerSessionState::Play => loop {
                    let data = self.data_consumer.recv().await;

                    match data {
                        Ok(val) => match val {
                            ChannelData::Audio { timestamp, data } => {
                                self.send_audio(data, timestamp).await?;
                            }
                            ChannelData::Video { timestamp, data } => {
                                self.send_video(data, timestamp).await?;
                            }
                            ChannelData::MetaData {} => {}
                        },
                        Err(err) => {}
                    }
                },
            }
        }

        //Ok(())
    }

    pub async fn send_set_chunk_size(&mut self) -> Result<(), SessionError> {
        let mut controlmessage = ControlMessages::new(AsyncBytesWriter::new(self.io.clone()));
        controlmessage.write_set_chunk_size(CHUNK_SIZE).await?;

        Ok(())
    }
    pub async fn send_audio(&mut self, data: BytesMut, timestamp: u32) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::AUDIO,
            chunk_type::TYPE_0,
            timestamp,
            data.len() as u32,
            msg_type_id::AUDIO,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub async fn send_video(&mut self, data: BytesMut, timestamp: u32) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::VIDEO,
            chunk_type::TYPE_0,
            timestamp,
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
                self.process_amf0_command_message(
                    msg_stream_id,
                    command_name,
                    transaction_id,
                    command_object,
                    others,
                )
                .await?
            }
            RtmpMessageData::AudioData { data } => {
                self.on_audio_data(data, timestamp)?;
            }
            RtmpMessageData::VideoData { data } => {
                self.on_video_data(data, timestamp)?;
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
                print!("connect .......");
                self.on_connect(&transaction_id, &obj).await?;
            }
            "createStream" => {
                self.on_create_stream(transaction_id).await?;
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
        control_message
            .write_window_acknowledgement_size(define::WINDOW_ACKNOWLEDGEMENT_SIZE)
            .await?;
        control_message
            .write_set_peer_bandwidth(
                define::PEER_BANDWIDTH,
                define::PeerBandWidthLimitType::DYNAMIC,
            )
            .await?;
        control_message.write_set_chunk_size(CHUNK_SIZE).await?;

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

    pub async fn on_create_stream(&mut self, transaction_id: &f64) -> Result<(), SessionError> {
        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection.create_stream_response(transaction_id, &define::STREAM_ID)?;

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

        self.subscribe_from_channels(stream_name.unwrap()).await?;
        self.state = ServerSessionState::Play;

        Ok(())
    }

    async fn subscribe_from_channels(&mut self, stream_name: String) -> Result<(), SessionError> {
        let (sender, receiver) = oneshot::channel();
        let subscribe_event = ChannelEvent::Subscribe {
            app_name: self.app_name.clone(),
            stream_name,
            responder: sender,
        };

        let rv = self.event_producer.send(subscribe_event);
        match rv {
            Err(_) => {
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                })
            }
            _ => {}
        }

        match receiver.await {
            Ok(consumer) => {
                self.data_consumer = consumer;
            }
            Err(_) => {}
        }
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

        //self.publish_to_channels(stream_name).await?;

        Ok(())
    }

    async fn publish_to_channels(&mut self, stream_name: String) -> Result<(), SessionError> {
        let (sender, receiver) = oneshot::channel();
        let publish_event = ChannelEvent::Publish {
            app_name: self.app_name.clone(),
            stream_name,
            responder: sender,
        };

        let rv = self.event_producer.send(publish_event);
        match rv {
            Err(_) => {
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                })
            }
            _ => {}
        }

        match receiver.await {
            Ok(producer) => {
                self.data_producer = producer;
            }
            Err(_) => {}
        }
        Ok(())
    }

    pub fn on_video_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        // let data = ChannelData::Video {
        //     timestamp: timestamp.clone(),
        //     data: data.clone(),
        // };

        // match self.data_producer.send(data) {
        //     Ok(size) => {}
        //     Err(_) => {
        //         return Err(SessionError {
        //             value: SessionErrorValue::SendChannelDataErr,
        //         })
        //     }
        // }

        Ok(())
    }

    pub fn on_audio_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        // let data = ChannelData::Audio {
        //     timestamp: timestamp.clone(),
        //     data: data.clone(),
        // };

        // match self.data_producer.send(data) {
        //     Ok(size) => {}
        //     Err(_) => {
        //         return Err(SessionError {
        //             value: SessionErrorValue::SendChannelDataErr,
        //         })
        //     }
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::io::Read;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use crate::chunk::unpacketizer::ChunkUnpacketizer;
    use crate::handshake::handshake::SimpleHandshakeServerSync;

    use crate::handshake::handshake::ServerHandshakeState;
    use crate::session::errors::SessionError;

    use super::ServerSessionState;
    use crate::chunk::unpacketizer::UnpackResult;
    use bytes::BytesMut;

    use crate::utils::print;
    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;
    pub struct RtmpProcessor {
        simple_handshaker: SimpleHandshakeServerSync,

        unpacketizer: ChunkUnpacketizer,

        stream: TcpStream,

        state: ServerSessionState,
    }

    impl RtmpProcessor {
        fn new(stream: TcpStream) -> Self {
            Self {
                simple_handshaker: SimpleHandshakeServerSync::new(),
                unpacketizer: ChunkUnpacketizer::new(),
                state: ServerSessionState::Handshake,
                stream: stream,
            }
        }
        fn handle_rtmp(&mut self, data: &[u8]) -> Result<(), SessionError> {
            let mut remaining_bytes: BytesMut = BytesMut::new();

            match self.state {
                ServerSessionState::Handshake => {
                    self.simple_handshaker.extend_data(data);
                    let data = self.simple_handshaker.handshake()?;

                    match self.simple_handshaker.state {
                        ServerHandshakeState::Finish => {
                            self.state = ServerSessionState::ReadChunk;
                            remaining_bytes = self.simple_handshaker.get_remaining_bytes();
                        }
                        _ => return Ok(()),
                    }
                }
                ServerSessionState::ReadChunk => {
                    if remaining_bytes.len() > 0 {
                        let bytes = std::mem::replace(&mut remaining_bytes, BytesMut::new());
                        self.unpacketizer.extend_data(&bytes[..]);
                    } else {
                        self.unpacketizer.extend_data(data);
                    }

                    let result = self.unpacketizer.read_chunks();

                    let rv = match result {
                        Ok(val) => val,
                        Err(err) => {
                            // return Err(SessionError {
                            //     value: SessionErrorValue::UnPackError(err),
                            // })
                            return Ok(());
                        }
                    };

                    match rv {
                        UnpackResult::Chunks(chunks) => {
                            for chunk_info in chunks.iter() {
                                let msg_stream_id = chunk_info.message_header.msg_streamd_id;
                                let timestamp = chunk_info.message_header.timestamp;
                            }
                        }
                        _ => {}
                    }
                }

                _ => {}
            }

            Ok(())
        }

        fn handle_client(&mut self) {
            // read 20 bytes at a time from stream echoing back to stream

            let mut remaining_bytes: BytesMut = BytesMut::new();
            loop {
                let mut read = [0; 10280];
                match self.stream.read(&mut read) {
                    Ok(n) => {
                        if n == 0 {
                            // connection was closed
                            break;
                        }

                        print::print_slice(&read[0..n]);

                        match self.state {
                            ServerSessionState::Handshake => {
                                self.simple_handshaker.extend_data(&read[0..n]);
                                let data = self.simple_handshaker.handshake();

                                match self.simple_handshaker.state {
                                    ServerHandshakeState::Finish => {
                                        self.state = ServerSessionState::ReadChunk;
                                        remaining_bytes =
                                            self.simple_handshaker.get_remaining_bytes();
                                    }
                                    _ => match data {
                                        Ok(b) => {
                                            self.stream.write(&b[..]);
                                            continue
                                        }

                                        _ => {}
                                    },
                                }
                            }
                            ServerSessionState::ReadChunk => {
                                if remaining_bytes.len() > 0 {
                                    let bytes =
                                        std::mem::replace(&mut remaining_bytes, BytesMut::new());
                                    self.unpacketizer.extend_data(&bytes[..]);
                                } else {
                                    self.unpacketizer.extend_data(&read[0..n]);
                                }

                                let result = self.unpacketizer.read_chunks();

                                let rv = match result {
                                    Ok(val) => val,
                                    Err(err) => {
                                        // return Err(SessionError {
                                        //     value: SessionErrorValue::UnPackError(err),
                                        // })
                                        continue;
                                    }
                                };

                                match rv {
                                    UnpackResult::Chunks(chunks) => {
                                        for chunk_info in chunks.iter() {
                                            let msg_stream_id =
                                                chunk_info.message_header.msg_streamd_id;
                                            let timestamp = chunk_info.message_header.timestamp;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            _ => {}
                        }
                    }
                    Err(err) => {
                        panic!(err);
                    }
                }
            }
        }
    }

    #[test]
    fn test_server_session() {
        let listener = TcpListener::bind("127.0.0.1:1935").unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || {
                        // let stream = Rc::new(RefCell::new(stream));
                        let mut processor = RtmpProcessor::new(stream);

                        processor.handle_client();
                    });
                }
                Err(_) => {
                    println!("Error");
                }
            }
        }
    }
}
