use serde::ser::SerializeStruct;

use {
    super::{
        define::{PublishType, SessionType, SubscribeType},
        errors::{SessionError, SessionErrorValue},
    },
    crate::{
        channels::define::{
            ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent,
            ChannelEventProducer,
        },
        chunk::{
            define::{chunk_type, csid_type},
            packetizer::ChunkPacketizer,
            ChunkInfo,
        },
        messages::define::msg_type_id,
    },
    bytes::BytesMut,
    bytesio::bytesio::BytesIO,
    serde::{Serialize, Serializer},
    std::{net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        sync::{mpsc, oneshot, Mutex},
        time::sleep,
    },
    uuid::Uuid,
};

#[derive(Debug, Serialize)]
pub struct NotifyInfo {
    pub request_url: String,
    pub remote_addr: String,
}
#[derive(Debug)]
pub struct SubscriberInfo {
    pub id: Uuid,
    pub sub_type: SubscribeType,
    pub notify_info: NotifyInfo,
}

impl Serialize for SubscriberInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SubscriberInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Debug)]
pub struct PublisherInfo {
    pub id: Uuid,
    pub sub_type: PublishType,
    pub notify_info: NotifyInfo,
}

impl Serialize for PublisherInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("PublisherInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}
pub struct Common {
    packetizer: ChunkPacketizer,

    data_consumer: ChannelDataConsumer,
    data_producer: ChannelDataProducer,

    event_producer: ChannelEventProducer,
    pub session_type: SessionType,

    /*save the client side socket connected to the SeverSession */
    remote_addr: Option<SocketAddr>,
    /*request URL from client*/
    pub request_url: String,
}

impl Common {
    pub fn new(
        net_io: Arc<Mutex<BytesIO>>,
        event_producer: ChannelEventProducer,
        session_type: SessionType,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        //only used for init,since I don't found a better way to deal with this.
        let (init_producer, init_consumer) = mpsc::unbounded_channel();

        Self {
            packetizer: ChunkPacketizer::new(Arc::clone(&net_io)),

            data_producer: init_producer,
            data_consumer: init_consumer,

            event_producer,
            session_type,
            remote_addr,
            request_url: String::default(),
        }
    }
    pub async fn send_channel_data(&mut self) -> Result<(), SessionError> {
        let mut retry_times = 0;
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                match data {
                    ChannelData::Audio { timestamp, data } => {
                        self.send_audio(data, timestamp).await?;
                    }
                    ChannelData::Video { timestamp, data } => {
                        self.send_video(data, timestamp).await?;
                    }
                    ChannelData::MetaData { timestamp, data } => {
                        self.send_metadata(data, timestamp).await?;
                    }
                }
            } else {
                retry_times += 1;
                log::debug!(
                    "send_channel_data: no data receives ,retry {} times!",
                    retry_times
                );

                if retry_times > 10 {
                    return Err(SessionError {
                        value: SessionErrorValue::NoMediaDataReceived,
                    });
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

    pub async fn send_metadata(
        &mut self,
        data: BytesMut,
        timestamp: u32,
    ) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::DATA_AMF0_AMF3,
            chunk_type::TYPE_0,
            timestamp,
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
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_producer.send(data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("send video err: {}", err);
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
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_producer.send(data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("receive audio err {}\n", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                });
            }
        }

        Ok(())
    }

    pub fn on_meta_data(
        &mut self,
        body: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let data = ChannelData::MetaData {
            timestamp: *timestamp,
            data: body.clone(),
        };

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

    fn get_subscriber_info(&mut self, sub_id: Uuid) -> SubscriberInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        match self.session_type {
            SessionType::Client => SubscriberInfo {
                id: sub_id,
                /*rtmp local client subscribe from local rtmp session
                and publish(relay) the rtmp steam to remote RTMP server*/
                sub_type: SubscribeType::PublisherRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
            SessionType::Server => SubscriberInfo {
                id: sub_id,
                /* rtmp player from remote clent */
                sub_type: SubscribeType::PlayerRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
        }
    }

    fn get_publisher_info(&mut self, sub_id: Uuid) -> PublisherInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        match self.session_type {
            SessionType::Client => PublisherInfo {
                id: sub_id,
                sub_type: PublishType::PushRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
            SessionType::Server => PublisherInfo {
                id: sub_id,
                sub_type: PublishType::SubscriberRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
        }
    }

    /*Begin to send data to retmote common player or local RTMP relay push client*/
    pub async fn subscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        log::info!(
            "subscribe_from_channels, app_name: {} stream_name: {} subscribe_id: {}",
            app_name,
            stream_name.clone(),
            sub_id
        );

        let mut retry_count: u8 = 0;

        loop {
            let (sender, receiver) = oneshot::channel();

            let subscribe_event = ChannelEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
                info: self.get_subscriber_info(sub_id),
                responder: sender,
            };
            let rv = self.event_producer.send(subscribe_event);

            if rv.is_err() {
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                });
            }

            match receiver.await {
                Ok(consumer) => {
                    self.data_consumer = consumer;
                    break;
                }
                Err(_) => {
                    if retry_count > 10 {
                        return Err(SessionError {
                            value: SessionErrorValue::SubscribeCountLimitReach,
                        });
                    }
                }
            }

            sleep(Duration::from_millis(800)).await;
            retry_count += 1;
        }

        Ok(())
    }

    pub async fn unsubscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        let subscribe_event = ChannelEvent::UnSubscribe {
            app_name,
            stream_name,
            info: self.get_subscriber_info(sub_id),
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}\n", err);
        }

        Ok(())
    }

    /*Begin to receive stream data from remote RTMP push client or local RTMP relay pull client*/
    pub async fn publish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        pub_id: Uuid,
    ) -> Result<(), SessionError> {
        let (sender, receiver) = oneshot::channel();
        let publish_event = ChannelEvent::Publish {
            app_name,
            stream_name,
            responder: sender,
            info: self.get_publisher_info(pub_id),
        };

        let rv = self.event_producer.send(publish_event);
        if rv.is_err() {
            return Err(SessionError {
                value: SessionErrorValue::ChannelEventSendErr,
            });
        }

        match receiver.await {
            Ok(producer) => {
                self.data_producer = producer;
            }
            Err(err) => {
                log::error!("publish_to_channels err{}\n", err);
            }
        }
        Ok(())
    }

    pub async fn unpublish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        pub_id: Uuid,
    ) -> Result<(), SessionError> {
        log::info!(
            "unpublish_to_channels, app_name:{}, stream_name:{}",
            app_name,
            stream_name
        );
        let unpublish_event = ChannelEvent::UnPublish {
            app_name: app_name.clone(),
            stream_name: stream_name.clone(),
            info: self.get_publisher_info(pub_id),
        };

        let rv = self.event_producer.send(unpublish_event);
        match rv {
            Err(_) => {
                log::error!(
                    "unpublish_to_channels error.app_name: {}, stream_name: {}",
                    app_name,
                    stream_name
                );
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                });
            }
            _ => {
                log::info!(
                    "unpublish_to_channels successfully.app_name: {}, stream_name: {}",
                    app_name,
                    stream_name
                );
            }
        }
        Ok(())
    }
}
