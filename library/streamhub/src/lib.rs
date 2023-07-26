pub mod define;
pub mod errors;
pub mod notify;
pub mod statistics;
pub mod stream;
pub mod utils;

use {
    crate::notify::Notifier,
    define::{
        AvStatisticSender, BroadcastEvent, BroadcastEventReceiver, BroadcastEventSender, FrameData,
        FrameDataReceiver, FrameDataSender, Information, PubSubInfo, StreamHubEvent,
        StreamHubEventReceiver, StreamHubEventSender, StreamStatisticSizeSender, SubscriberInfo,
        TStreamHandler, TransmitterEvent, TransmitterEventConsumer, TransmitterEventProducer,
    },
    errors::{ChannelError, ChannelErrorValue},
    std::collections::HashMap,
    std::sync::Arc,
    stream::StreamIdentifier,
    tokio::sync::{broadcast, mpsc, mpsc::UnboundedReceiver},
    utils::Uuid,
};

//receive data from ChannelsManager and send to players/subscribers
pub struct Transmitter {
    //used for receiving Audio/Video data
    data_consumer: FrameDataReceiver,
    //used for receiving event
    event_consumer: TransmitterEventConsumer,
    //used for sending audio/video data to players/subscribers
    subscriberid_to_producer: HashMap<Uuid, FrameDataSender>,
    stream_handler: Arc<dyn TStreamHandler>,
}

impl Transmitter {
    fn new(
        data_consumer: UnboundedReceiver<FrameData>,
        event_consumer: UnboundedReceiver<TransmitterEvent>,
        h: Arc<dyn TStreamHandler>,
    ) -> Self {
        Self {
            data_consumer,
            event_consumer,
            subscriberid_to_producer: HashMap::new(),
            stream_handler: h,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError> {
        loop {
            tokio::select! {
                data = self.event_consumer.recv() => {
                    if let Some(val) = data {
                        match val {
                            TransmitterEvent::Subscribe { sender, info } => {
                                self.stream_handler
                                    .send_prior_data(sender.clone(), info.sub_type)
                                    .await?;

                                self.subscriberid_to_producer.insert(info.id, sender);
                            }
                            TransmitterEvent::UnSubscribe { info } => {
                                self.subscriberid_to_producer.remove(&info.id);
                            }
                            TransmitterEvent::UnPublish {} => {
                                return Ok(());
                            }
                            TransmitterEvent::Api { sender } => {
                                if let Some(avstatistic_data) = self.stream_handler.get_statistic_data().await {
                                    if let Err(err) = sender.send(avstatistic_data) {
                                        log::info!("Transmitter send avstatistic data err: {}", err);
                                    }
                                }
                            }
                            TransmitterEvent::Request {sender} =>{
                                self.stream_handler.send_information(sender).await;
                            }
                        }
                    }

                }
                data = self.data_consumer.recv() => {
                    if let Some(val) = data {
                        match val {
                            FrameData::MetaData { timestamp:_, data:_ } => {

                            }
                            FrameData::Audio { timestamp, data } => {
                                let data = FrameData::Audio {
                                    timestamp,
                                    data: data.clone(),
                                };

                                for (_, v) in self.subscriberid_to_producer.iter() {
                                    if let Err(audio_err) = v.send(data.clone()).map_err(|_| ChannelError {
                                        value: ChannelErrorValue::SendAudioError,
                                    }) {
                                        log::error!("Transmiter send error: {}", audio_err);
                                    }
                                }
                            }
                            FrameData::Video { timestamp, data } => {
                                let data = FrameData::Video {
                                    timestamp,
                                    data: data.clone(),
                                };
                                for (_, v) in self.subscriberid_to_producer.iter() {
                                    if let Err(video_err) = v.send(data.clone()).map_err(|_| ChannelError {
                                        value: ChannelErrorValue::SendVideoError,
                                    }) {
                                        log::error!("Transmiter send error: {}", video_err);
                                    }
                                }
                            }
                            FrameData::MediaInfo{media_info: _} =>{

                            }
                        }
                    }
                }
            }
        }

        //Ok(())
    }
}

pub struct StreamsHub {
    //app_name to stream_name to producer
    streams: HashMap<StreamIdentifier, TransmitterEventProducer>,
    //save info to kick off client
    streams_info: HashMap<Uuid, PubSubInfo>,
    //event is consumed in Channels, produced from other rtmp sessions
    hub_event_receiver: StreamHubEventReceiver,
    //event is produced from other rtmp sessions
    hub_event_sender: StreamHubEventSender,
    //client_event_producer: client_event_producer
    client_event_producer: BroadcastEventSender,
    //The rtmp static push/pull and the hls transfer is triggered actively,
    //add a control switches separately.
    rtmp_push_enabled: bool,
    rtmp_remuxer_enabled: bool,
    //enable rtmp pull
    rtmp_pull_enabled: bool,
    //enable hls
    hls_enabled: bool,
    //http notifier on sub/pub event
    notifier: Option<Notifier>,
}

impl StreamsHub {
    pub fn new(notifier: Option<Notifier>) -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        let (client_producer, _) = broadcast::channel(100);

        Self {
            streams: HashMap::new(),
            streams_info: HashMap::new(),
            hub_event_receiver: event_consumer,
            hub_event_sender: event_producer,
            client_event_producer: client_producer,
            rtmp_push_enabled: false,
            rtmp_pull_enabled: false,
            rtmp_remuxer_enabled: false,
            hls_enabled: false,
            notifier,
        }
    }
    pub async fn run(&mut self) {
        self.event_loop().await;
    }

    pub fn set_rtmp_push_enabled(&mut self, enabled: bool) {
        self.rtmp_push_enabled = enabled;
    }

    pub fn set_rtmp_pull_enabled(&mut self, enabled: bool) {
        self.rtmp_pull_enabled = enabled;
    }

    pub fn set_rtmp_remuxer_enabled(&mut self, enabled: bool) {
        self.rtmp_remuxer_enabled = enabled;
    }

    pub fn set_hls_enabled(&mut self, enabled: bool) {
        self.hls_enabled = enabled;
    }

    pub fn get_hub_event_sender(&mut self) -> StreamHubEventSender {
        self.hub_event_sender.clone()
    }

    pub fn get_client_event_consumer(&mut self) -> BroadcastEventReceiver {
        self.client_event_producer.subscribe()
    }

    pub async fn event_loop(&mut self) {
        while let Some(message) = self.hub_event_receiver.recv().await {
            let event_serialize_str = if let Ok(data) = serde_json::to_string(&message) {
                log::info!("event data: {}", data);
                data
            } else {
                String::from("empty body")
            };

            match message {
                StreamHubEvent::Publish {
                    identifier,
                    receiver,
                    info,
                    stream_handler,
                } => {
                    let rv = self.publish(identifier.clone(), receiver, stream_handler);
                    match rv {
                        Ok(()) => {
                            if let Some(notifier) = &self.notifier {
                                notifier.on_publish_notify(event_serialize_str).await;
                            }
                            self.streams_info
                                .insert(info.id, PubSubInfo::Publish { identifier });
                        }
                        Err(err) => {
                            log::error!("event_loop Publish err: {}\n", err);
                            continue;
                        }
                    }
                }

                StreamHubEvent::UnPublish {
                    identifier,
                    info: _,
                } => {
                    if let Err(err) = self.unpublish(&identifier) {
                        log::error!(
                            "event_loop Unpublish err: {} with identifier: {} \n",
                            err,
                            identifier
                        );
                    }

                    if let Some(notifier) = &self.notifier {
                        notifier.on_unpublish_notify(event_serialize_str).await;
                    }
                }
                StreamHubEvent::Subscribe {
                    identifier,
                    info,
                    sender,
                } => {
                    let sub_id = info.id;
                    let info_clone = info.clone();
                    let rv = self.subscribe(&identifier, info_clone, sender).await;
                    match rv {
                        Ok(()) => {
                            if let Some(notifier) = &self.notifier {
                                notifier.on_play_notify(event_serialize_str).await;
                            }

                            self.streams_info.insert(
                                sub_id,
                                PubSubInfo::Subscribe {
                                    identifier,
                                    sub_info: info,
                                },
                            );
                        }
                        Err(err) => {
                            log::error!("event_loop Subscribe error: {}", err);
                            continue;
                        }
                    }
                }
                StreamHubEvent::UnSubscribe { identifier, info } => {
                    if self.unsubscribe(&identifier, info).is_ok() {
                        if let Some(notifier) = &self.notifier {
                            notifier.on_stop_notify(event_serialize_str).await;
                        }
                    }
                }

                StreamHubEvent::ApiStatistic {
                    data_sender,
                    size_sender,
                } => {
                    if let Err(err) = self.api_statistic(data_sender, size_sender) {
                        log::error!("event_loop api error: {}", err);
                    }
                }
                StreamHubEvent::ApiKickClient { id } => {
                    self.api_kick_off_client(id);

                    if let Some(notifier) = &self.notifier {
                        notifier.on_unpublish_notify(event_serialize_str).await;
                    }
                }
                StreamHubEvent::Request { identifier, sender } => {
                    if let Err(err) = self.request(&identifier, sender) {
                        log::error!("event_loop request error: {}", err);
                    }
                }
            }
        }
    }

    fn request(
        &mut self,
        identifier: &StreamIdentifier,
        sender: mpsc::UnboundedSender<Information>,
    ) -> Result<(), ChannelError> {
        if let Some(producer) = self.streams.get_mut(identifier) {
            let event = TransmitterEvent::Request { sender };
            log::info!("Request:  stream identifier: {}", identifier);
            producer.send(event).map_err(|_| ChannelError {
                value: ChannelErrorValue::SendError,
            })?;
        }
        Ok(())
    }

    fn api_statistic(
        &mut self,
        data_sender: AvStatisticSender,
        size_sender: StreamStatisticSizeSender,
    ) -> Result<(), ChannelError> {
        let mut stream_count: usize = 0;
        for v in self.streams.values() {
            stream_count += 1;
            if let Err(err) = v.send(TransmitterEvent::Api {
                sender: data_sender.clone(),
            }) {
                log::error!("TransmitterEvent  api send data err: {}", err);
                return Err(ChannelError {
                    value: ChannelErrorValue::SendError,
                });
            }
        }

        if let Err(err) = size_sender.send(stream_count) {
            log::error!("TransmitterEvent api send size err: {}", err);
            return Err(ChannelError {
                value: ChannelErrorValue::SendError,
            });
        }

        Ok(())
    }

    fn api_kick_off_client(&mut self, uid: Uuid) {
        let info = if let Some(info) = self.streams_info.get(&uid) {
            info.clone()
        } else {
            return;
        };

        match info {
            PubSubInfo::Publish { identifier } => {
                if let Err(err) = self.unpublish(&identifier) {
                    log::error!(
                        "event_loop ApiKickClient pub err: {} with identifier: {} \n",
                        err,
                        identifier
                    );
                }
            }
            PubSubInfo::Subscribe {
                identifier,
                sub_info,
            } => {
                if let Err(err) = self.unsubscribe(&identifier, sub_info) {
                    log::error!(
                        "event_loop ApiKickClient pub err: {} with identifier: {}\n",
                        err,
                        identifier
                    );
                }
            }
        }
    }

    //player subscribe a stream
    pub async fn subscribe(
        &mut self,
        identifer: &StreamIdentifier,
        sub_info: SubscriberInfo,
        sender: FrameDataSender,
    ) -> Result<(), ChannelError> {
        if let Some(producer) = self.streams.get_mut(identifer) {
            let event = TransmitterEvent::Subscribe {
                sender,
                info: sub_info,
            };
            log::info!("subscribe:  stream identifier: {}", identifer);
            producer.send(event).map_err(|_| ChannelError {
                value: ChannelErrorValue::SendError,
            })?;

            return Ok(());
        }

        if self.rtmp_pull_enabled {
            log::info!("subscribe: try to pull stream, identifier: {}", identifer);

            let client_event = BroadcastEvent::Subscribe {
                identifier: identifer.clone(),
            };

            //send subscribe info to pull clients
            self.client_event_producer
                .send(client_event)
                .map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
        }

        Err(ChannelError {
            value: ChannelErrorValue::NoAppOrStreamName,
        })
    }

    pub fn unsubscribe(
        &mut self,
        identifer: &StreamIdentifier,
        sub_info: SubscriberInfo,
    ) -> Result<(), ChannelError> {
        match self.streams.get_mut(identifer) {
            Some(producer) => {
                log::info!("unsubscribe....:{}", identifer);
                let event = TransmitterEvent::UnSubscribe { info: sub_info };
                producer.send(event).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
            }
            None => {
                return Err(ChannelError {
                    value: ChannelErrorValue::NoAppName,
                })
            }
        }

        Ok(())
    }

    //publish a stream
    pub fn publish(
        &mut self,
        identifier: StreamIdentifier,
        receiver: FrameDataReceiver,
        handler: Arc<dyn TStreamHandler>,
    ) -> Result<(), ChannelError> {
        if self.streams.get(&identifier).is_some() {
            return Err(ChannelError {
                value: ChannelErrorValue::Exists,
            });
        }

        let (event_publisher, event_consumer) = mpsc::unbounded_channel();
        let mut transmitter = Transmitter::new(receiver, event_consumer, handler);

        let identifier_clone = identifier.clone();
        tokio::spawn(async move {
            if let Err(err) = transmitter.run().await {
                log::error!(
                    "transmiter run error, idetifier: {}, error: {}",
                    identifier_clone,
                    err,
                );
            } else {
                log::info!("transmiter exits: idetifier: {}", identifier_clone);
            }
        });

        self.streams.insert(identifier.clone(), event_publisher);

        if self.rtmp_push_enabled || self.hls_enabled || self.rtmp_remuxer_enabled {
            let client_event = BroadcastEvent::Publish { identifier };

            //send publish info to push clients
            self.client_event_producer
                .send(client_event)
                .map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
        }

        Ok(())
    }

    fn unpublish(&mut self, identifier: &StreamIdentifier) -> Result<(), ChannelError> {
        match self.streams.get_mut(identifier) {
            Some(producer) => {
                let event = TransmitterEvent::UnPublish {};
                producer.send(event).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
                self.streams.remove(identifier);
                log::info!("unpublish remove stream, stream identifier: {}", identifier);
            }
            None => {
                return Err(ChannelError {
                    value: ChannelErrorValue::NoAppName,
                })
            }
        }

        Ok(())
    }
}
