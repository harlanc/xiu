use streamhub::define::DataSender;
use tokio::sync::oneshot;

use {
    super::{
        define::SessionType,
        errors::{SessionError, SessionErrorValue},
    },
    crate::{
        cache::errors::CacheError,
        cache::Cache,
        chunk::{
            define::{chunk_type, csid_type},
            packetizer::ChunkPacketizer,
            ChunkInfo,
        },
        messages::define::msg_type_id,
    },
    async_trait::async_trait,
    bytes::BytesMut,
    std::fmt,
    std::{net::SocketAddr, sync::Arc},
    streamhub::{
        define::{
            FrameData, FrameDataReceiver, FrameDataSender, InformationSender, NotifyInfo,
            PublishType, PublisherInfo, StreamHubEvent, StreamHubEventSender, SubscribeType,
            SubscriberInfo, TStreamHandler,
        },
        errors::{StreamHubError, StreamHubErrorValue},
        statistics::StreamStatistics,
        stream::StreamIdentifier,
        utils::Uuid,
    },
    tokio::sync::{mpsc, Mutex},
};

pub struct Common {
    //only Server Subscriber or Client Publisher needs to send out trunck data.
    packetizer: Option<ChunkPacketizer>,

    data_receiver: FrameDataReceiver,
    data_sender: FrameDataSender,

    event_producer: StreamHubEventSender,
    pub session_type: SessionType,

    /*save the client side socket connected to the SeverSession */
    remote_addr: Option<SocketAddr>,
    /*request URL from client*/
    pub request_url: String,
    pub stream_handler: Arc<RtmpStreamHandler>,
}

impl Common {
    pub fn new(
        packetizer: Option<ChunkPacketizer>,
        event_producer: StreamHubEventSender,
        session_type: SessionType,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        //only used for init,since I don't found a better way to deal with this.
        let (init_producer, init_consumer) = mpsc::unbounded_channel();

        Self {
            packetizer,

            data_sender: init_producer,
            data_receiver: init_consumer,

            event_producer,
            session_type,
            remote_addr,
            request_url: String::default(),
            stream_handler: Arc::new(RtmpStreamHandler::new()),
            //cache: None,
        }
    }
    pub async fn send_channel_data(&mut self) -> Result<(), SessionError> {
        let mut retry_times = 0;
        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    FrameData::Audio { timestamp, data } => {
                        self.send_audio(data, timestamp).await?;
                    }
                    FrameData::Video { timestamp, data } => {
                        self.send_video(data, timestamp).await?;
                    }
                    FrameData::MetaData { timestamp, data } => {
                        self.send_metadata(data, timestamp).await?;
                    }
                    _ => {}
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

        if let Some(packetizer) = &mut self.packetizer {
            packetizer.write_chunk(&mut chunk_info).await?;
        }

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

        if let Some(packetizer) = &mut self.packetizer {
            packetizer.write_chunk(&mut chunk_info).await?;
        }

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

        if let Some(packetizer) = &mut self.packetizer {
            packetizer.write_chunk(&mut chunk_info).await?;
        }

        Ok(())
    }

    pub async fn on_video_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = FrameData::Video {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("send video err: {}", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendFrameDataErr,
                });
            }
        }

        self.stream_handler
            .save_video_data(data, *timestamp)
            .await?;

        Ok(())
    }

    pub async fn on_audio_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = FrameData::Audio {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("receive audio err {}", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendFrameDataErr,
                });
            }
        }

        self.stream_handler
            .save_audio_data(data, *timestamp)
            .await?;

        Ok(())
    }

    pub async fn on_meta_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = FrameData::MetaData {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(_) => {
                return Err(SessionError {
                    value: SessionErrorValue::SendFrameDataErr,
                })
            }
        }

        self.stream_handler.save_metadata(data, *timestamp).await;

        Ok(())
    }

    fn get_subscriber_info(&mut self, sub_id: Uuid) -> SubscriberInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        let sub_type = match self.session_type {
            SessionType::Client => SubscribeType::PublisherRtmp,
            SessionType::Server => SubscribeType::PlayerRtmp,
        };

        SubscriberInfo {
            id: sub_id,
            /*rtmp local client subscribe from local rtmp session
            and publish(relay) the rtmp steam to remote RTMP server*/
            sub_type,
            sub_data_type: streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr,
            },
        }
    }

    fn get_publisher_info(&mut self, sub_id: Uuid) -> PublisherInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        let pub_type = match self.session_type {
            SessionType::Client => PublishType::RelayRtmp,
            SessionType::Server => PublishType::PushRtmp,
        };

        PublisherInfo {
            id: sub_id,
            pub_type,
            pub_data_type: streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr,
            },
        }
    }

    /*Subscribe from local channels and then send data to retmote common player or local RTMP relay push client*/
    pub async fn subscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        log::info!(
            "subscribe_from_channels, app_name: {} stream_name: {} subscribe_id: {}",
            app_name,
            stream_name,
            sub_id
        );

        let identifier = StreamIdentifier::Rtmp {
            app_name,
            stream_name,
        };

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: self.get_subscriber_info(sub_id),
            result_sender: event_result_sender,
        };
        let rv = self.event_producer.send(subscribe_event);

        if rv.is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let recv = event_result_receiver.await??;
        self.data_receiver = recv.frame_receiver.unwrap();

        Ok(())
    }

    pub async fn unsubscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        let identifier = StreamIdentifier::Rtmp {
            app_name,
            stream_name,
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: self.get_subscriber_info(sub_id),
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}", err);
        }

        Ok(())
    }

    /*Begin to receive stream data from remote RTMP push client or local RTMP relay pull client*/
    pub async fn publish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        pub_id: Uuid,
        gop_num: usize,
    ) -> Result<(), SessionError> {
        self.stream_handler
            .set_cache(Cache::new(app_name.clone(), stream_name.clone(), gop_num))
            .await;

        let (event_result_sender, event_result_receiver) = oneshot::channel();
        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::Rtmp {
                app_name,
                stream_name,
            },
            info: self.get_publisher_info(pub_id),
            stream_handler: self.stream_handler.clone(),
            result_sender: event_result_sender,
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let result = event_result_receiver.await??;
        self.data_sender = result.0.unwrap();
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
        let unpublish_event = StreamHubEvent::UnPublish {
            identifier: StreamIdentifier::Rtmp {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
            },
            info: self.get_publisher_info(pub_id),
        };

        match self.event_producer.send(unpublish_event) {
            Err(_) => {
                log::error!(
                    "unpublish_to_channels error.app_name: {}, stream_name: {}",
                    app_name,
                    stream_name
                );
                return Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
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

#[derive(Default)]
pub struct RtmpStreamHandler {
    /*cache is used to save RTMP sequence/gops/meta data
    which needs to be send to client(player) */
    /*The cache will be used in different threads(save
    cache in one thread and send cache data to different clients
    in other threads) */
    pub cache: Mutex<Option<Cache>>,
}

impl RtmpStreamHandler {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }

    pub async fn set_cache(&self, cache: Cache) {
        *self.cache.lock().await = Some(cache);
    }

    pub async fn save_video_data(
        &self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_video_data(chunk_body, timestamp).await?;
        }
        Ok(())
    }

    pub async fn save_audio_data(
        &self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_audio_data(chunk_body, timestamp).await?;
        }
        Ok(())
    }

    pub async fn save_metadata(&self, chunk_body: &BytesMut, timestamp: u32) {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_metadata(chunk_body, timestamp);
        }
    }
}

#[async_trait]
impl TStreamHandler for RtmpStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        let sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        if let Some(cache) = &mut *self.cache.lock().await {
            if let Some(meta_body_data) = cache.get_metadata() {
                sender.send(meta_body_data).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
            }
            if let Some(audio_seq_data) = cache.get_audio_seq() {
                sender.send(audio_seq_data).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
            }
            if let Some(video_seq_data) = cache.get_video_seq() {
                sender.send(video_seq_data).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
            }
            match sub_type {
                SubscribeType::PlayerRtmp
                | SubscribeType::PlayerHttpFlv
                | SubscribeType::PlayerHls
                | SubscribeType::GenerateHls => {
                    if let Some(gops_data) = cache.get_gops_data() {
                        for gop in gops_data {
                            for channel_data in gop.get_frame_data() {
                                sender.send(channel_data).map_err(|_| StreamHubError {
                                    value: StreamHubErrorValue::SendError,
                                })?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StreamStatistics> {
        if let Some(cache) = &mut *self.cache.lock().await {
            return Some(cache.av_statistics.get_avstatistic_data().await);
        }

        None
    }

    async fn send_information(&self, _: InformationSender) {}
}

impl fmt::Debug for Common {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "S2 {{ member: {:?} }}", self.request_url)
    }
}
