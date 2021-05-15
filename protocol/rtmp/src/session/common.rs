use {
    super::{
        define::{SessionSubType, SessionType},
        errors::{SessionError, SessionErrorValue},
    },
    crate::{
        amf0::Amf0ValueType,
        channels::define::{
            ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent,
            ChannelEventProducer,
        },
        chunk::{
            define::{chunk_type, csid_type, CHUNK_SIZE},
            packetizer::ChunkPacketizer,
            unpacketizer::{ChunkUnpacketizer, UnpackResult},
            ChunkInfo,
        },
        config,
        handshake::handshake::{ServerHandshakeState, SimpleHandshakeServer},
        messages::{
            define::{msg_type_id, RtmpMessageData},
            parser::MessageParser,
        },
        netconnection::commands::NetConnection,
        netstream::writer::NetStreamWriter,
        protocol_control_messages::writer::ProtocolControlMessagesWriter,
        user_control_messages::writer::EventMessagesWriter,
        utils::print::print,
    },
    bytes::BytesMut,
    networkio::{
        bytes_writer::{AsyncBytesWriter, BytesWriter},
        networkio::NetworkIO,
    },
    std::{collections::HashMap, sync::Arc},
    tokio::{
        net::TcpStream,
        sync::{mpsc, oneshot, Mutex},
    },
};

pub struct SessionInfo {
    pub session_id: u64,
    pub session_sub_type: SessionSubType,
}
pub struct Common {
    packetizer: ChunkPacketizer,

    data_consumer: ChannelDataConsumer,
    data_producer: ChannelDataProducer,

    event_producer: ChannelEventProducer,
    session_type: SessionType,
}

impl Common {
    pub fn new(
        net_io: Arc<Mutex<NetworkIO>>,
        event_producer: ChannelEventProducer,
        session_type: SessionType,
    ) -> Self {
        //only used for init,since I don't found a better way to deal with this.
        let (init_producer, init_consumer) = mpsc::unbounded_channel();

        Self {
            packetizer: ChunkPacketizer::new(Arc::clone(&net_io)),

            data_producer: init_producer,
            data_consumer: init_consumer,

            event_producer,
            session_type,
        }
    }
    pub async fn send_channel_data(&mut self) -> Result<(), SessionError> {
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                match data {
                    ChannelData::Audio { timestamp, data } => {
                        self.send_audio(data, timestamp).await?;
                    }
                    ChannelData::Video { timestamp, data } => {
                        self.send_video(data, timestamp).await?;
                    }
                    ChannelData::MetaData { body } => {
                        self.send_metadata(body).await?;
                    }
                }
            }
        }
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

    pub async fn send_metadata(&mut self, data: BytesMut) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::DATA_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type_id::DATA_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }

    pub fn on_video_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let data = ChannelData::Video {
            timestamp: timestamp.clone(),
            data: data.clone(),
        };

        //print!("receive video data\n");
        match self.data_producer.send(data) {
            Ok(_) => {}
            Err(err) => {
                print!("send video err {}\n", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                });
            }
        }

        Ok(())
    }

    pub fn on_audio_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let data = ChannelData::Audio {
            timestamp: timestamp.clone(),
            data: data.clone(),
        };

        match self.data_producer.send(data) {
            Ok(_) => {}
            Err(err) => {
                print!("receive audio err {}\n", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                });
            }
        }

        Ok(())
    }

    pub fn on_meta_data(&mut self, body: &mut BytesMut) -> Result<(), SessionError> {
        let data = ChannelData::MetaData { body: body.clone() };

        match self.data_producer.send(data) {
            Ok(_) => {}
            Err(_) => {
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                })
            }
        }

        Ok(())
    }

    fn get_session_info(&mut self, session_id: u64) -> SessionInfo {
        match self.session_type {
            SessionType::Client => SessionInfo {
                session_id: session_id,
                session_sub_type: SessionSubType::Publisher,
            },
            SessionType::Server => SessionInfo {
                session_id: session_id,
                session_sub_type: SessionSubType::Player,
            },
        }
    }

    pub async fn subscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        session_id: u64,
    ) -> Result<(), SessionError> {
        print!(
            "subscribe info............{} {} {}\n",
            app_name,
            stream_name.clone(),
            session_id
        );

        let (sender, receiver) = oneshot::channel();

        let subscribe_event = ChannelEvent::Subscribe {
            app_name: app_name,
            stream_name,
            session_info: self.get_session_info(session_id),
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

    pub async fn unsubscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        session_id: u64,
    ) -> Result<(), SessionError> {
        let subscribe_event = ChannelEvent::UnSubscribe {
            app_name,
            stream_name,
            session_info: self.get_session_info(session_id),
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            print!("unsubscribe_from_channels err {}\n", err)
        }

        Ok(())
    }

    pub async fn publish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
    ) -> Result<(), SessionError> {
        let (sender, receiver) = oneshot::channel();
        let publish_event = ChannelEvent::Publish {
            app_name,
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
                //print!("set producer before\n");
                self.data_producer = producer;
                //print!("set producer after\n");
            }
            Err(err) => {
                print!("publish_to_channels err{}\n", err)
            }
        }
        Ok(())
    }

    pub async fn unpublish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
    ) -> Result<(), SessionError> {
        let unpublish_event = ChannelEvent::UnPublish {
            app_name,
            stream_name,
        };

        let rv = self.event_producer.send(unpublish_event);
        match rv {
            Err(_) => {
                println!("unpublish_to_channels error.");
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                });
            }
            _ => {
                println!("unpublish_to_channels successfully.")
            }
        }
        Ok(())
    }
}
