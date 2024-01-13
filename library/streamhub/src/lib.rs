use define::{FrameDataReceiver, PacketDataReceiver, PacketDataSender};

use crate::define::PacketData;

pub mod define;
pub mod errors;
pub mod notify;
pub mod statistics;
pub mod stream;
pub mod utils;

use {
    crate::notify::Notifier,
    define::{
        AvStatisticSender, BroadcastEvent, BroadcastEventReceiver, BroadcastEventSender,
        DataReceiver, DataSender, FrameData, FrameDataSender, Information, PubSubInfo,
        StreamHubEvent, StreamHubEventReceiver, StreamHubEventSender, StreamStatisticSizeSender,
        SubscribeType, SubscriberInfo, TStreamHandler, TransmitterEvent, TransmitterEventReceiver,
        TransmitterEventSender,
    },
    errors::{ChannelError, ChannelErrorValue},
    std::collections::HashMap,
    std::sync::Arc,
    stream::StreamIdentifier,
    tokio::sync::{broadcast, mpsc, mpsc::UnboundedReceiver, Mutex},
    utils::Uuid,
};

//receive data from ChannelsManager and send to players/subscribers
pub struct Transmitter {
    //used for receiving Audio/Video data from publishers
    data_receiver: DataReceiver,
    //used for receiving event
    event_receiver: TransmitterEventReceiver,
    //used for sending audio/video frame data to players/subscribers
    id_to_frame_sender: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    //used for sending audio/video packet data to players/subscribers
    id_to_packet_sender: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
    stream_handler: Arc<dyn TStreamHandler>,
}

impl Transmitter {
    fn new(
        data_receiver: DataReceiver,
        event_receiver: UnboundedReceiver<TransmitterEvent>,
        h: Arc<dyn TStreamHandler>,
    ) -> Self {
        Self {
            data_receiver,
            event_receiver,
            id_to_frame_sender: Arc::new(Mutex::new(HashMap::new())),
            id_to_packet_sender: Arc::new(Mutex::new(HashMap::new())),
            stream_handler: h,
        }
    }

    pub async fn receive_frame_data_loop(
        mut exit: broadcast::Receiver<()>,
        mut receiver: FrameDataReceiver,
        frame_senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv() => {
                        if let Some(val) = data {
                            match val {
                                FrameData::MetaData {
                                    timestamp: _,
                                    data: _,
                                } => {}
                                FrameData::Audio { timestamp, data } => {
                                    let data = FrameData::Audio {
                                        timestamp,
                                        data: data.clone(),
                                    };

                                    for (_, v) in frame_senders.lock().await.iter() {
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
                                    for (_, v) in frame_senders.lock().await.iter() {
                                        if let Err(video_err) = v.send(data.clone()).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendVideoError,
                                        }) {
                                            log::error!("Transmiter send error: {}", video_err);
                                        }
                                    }
                                }
                                FrameData::MediaInfo { media_info: _ } => {}
                            }
                        }
                    }
                    _ = exit.recv()=>{
                        break;
                    }
                }
            }
        });
    }

    pub async fn receive_packet_data_loop(
        mut exit: broadcast::Receiver<()>,
        mut receiver: PacketDataReceiver,
        packet_senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv() => {
                        if let Some(val) = data {
                            match val {

                                PacketData::Audio { timestamp, data } => {
                                    let data = PacketData::Audio {
                                        timestamp,
                                        data: data.clone(),
                                    };

                                    for (_, v) in packet_senders.lock().await.iter() {
                                        if let Err(audio_err) = v.send(data.clone()).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendAudioError,
                                        }) {
                                            log::error!("Transmiter send error: {}", audio_err);
                                        }
                                    }
                                }
                                PacketData::Video { timestamp, data } => {
                                    let data = PacketData::Video {
                                        timestamp,
                                        data: data.clone(),
                                    };
                                    for (_, v) in packet_senders.lock().await.iter() {
                                        if let Err(video_err) = v.send(data.clone()).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendVideoError,
                                        }) {
                                            log::error!("Transmiter send error: {}", video_err);
                                        }
                                    }
                                }

                            }
                        }
                    }
                    _ = exit.recv()=>{
                        break;
                    }
                }
            }
        });
    }
    pub async fn receive_event_loop(
        stream_handler: Arc<dyn TStreamHandler>,
        exit: broadcast::Sender<()>,
        mut receiver: TransmitterEventReceiver,
        packet_senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
        frame_senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    ) {
        tokio::spawn(async move {
            loop {
                if let Some(val) = receiver.recv().await {
                    match val {
                        TransmitterEvent::Subscribe { sender, info } => {
                            if let Err(err) = stream_handler
                                .send_prior_data(sender.clone(), info.sub_type)
                                .await
                            {
                                log::error!("receive_event_loop send_prior_data err: {}", err);
                                break;
                            }
                            match sender {
                                DataSender::Frame {
                                    sender: frame_sender,
                                } => {
                                    frame_senders.lock().await.insert(info.id, frame_sender);
                                }
                                DataSender::Packet {
                                    sender: packet_sender,
                                } => {
                                    packet_senders.lock().await.insert(info.id, packet_sender);
                                }
                            }
                        }
                        TransmitterEvent::UnSubscribe { info } => match info.sub_type {
                            SubscribeType::PlayerRtp | SubscribeType::PlayerWebrtc => {
                                packet_senders.lock().await.remove(&info.id);
                            }
                            _ => {
                                frame_senders.lock().await.remove(&info.id);
                            }
                        },
                        TransmitterEvent::UnPublish {} => {
                            if let Err(err) = exit.send(()) {
                                log::error!("TransmitterEvent::UnPublish send error: {}", err);
                            }
                            break;
                        }
                        TransmitterEvent::Api { sender } => {
                            if let Some(avstatistic_data) =
                                stream_handler.get_statistic_data().await
                            {
                                if let Err(err) = sender.send(avstatistic_data) {
                                    log::info!("Transmitter send avstatistic data err: {}", err);
                                }
                            }
                        }
                        TransmitterEvent::Request { sender } => {
                            stream_handler.send_information(sender).await;
                        }
                    }
                }
            }
        });
    }

    pub async fn run(self) -> Result<(), ChannelError> {
        let (tx, _) = broadcast::channel::<()>(1);

        if let Some(receiver) = self.data_receiver.frame_receiver {
            Self::receive_frame_data_loop(
                tx.subscribe(),
                receiver,
                self.id_to_frame_sender.clone(),
            )
            .await;
        }

        if let Some(receiver) = self.data_receiver.packet_receiver {
            Self::receive_packet_data_loop(
                tx.subscribe(),
                receiver,
                self.id_to_packet_sender.clone(),
            )
            .await;
        }

        Self::receive_event_loop(
            self.stream_handler,
            tx,
            self.event_receiver,
            self.id_to_packet_sender,
            self.id_to_frame_sender,
        )
        .await;

        Ok(())
    }
}

pub struct StreamsHub {
    //app_name to stream_name to producer
    streams: HashMap<StreamIdentifier, TransmitterEventSender>,
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
                    info,
                    result_sender,
                    stream_handler,
                } => {
                    let (frame_sender, packet_sender, receiver) = match info.pub_data_type {
                        define::PubDataType::Frame => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                Some(sender_chan),
                                None,
                                DataReceiver {
                                    frame_receiver: Some(receiver_chan),
                                    packet_receiver: None,
                                },
                            )
                        }
                        define::PubDataType::Packet => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                None,
                                Some(sender_chan),
                                DataReceiver {
                                    frame_receiver: None,
                                    packet_receiver: Some(receiver_chan),
                                },
                            )
                        }
                        define::PubDataType::Both => {
                            let (sender_frame_chan, receiver_frame_chan) =
                                mpsc::unbounded_channel();
                            let (sender_packet_chan, receiver_packet_chan) =
                                mpsc::unbounded_channel();

                            (
                                Some(sender_frame_chan),
                                Some(sender_packet_chan),
                                DataReceiver {
                                    frame_receiver: Some(receiver_frame_chan),
                                    packet_receiver: Some(receiver_packet_chan),
                                },
                            )
                        }
                    };

                    let result = match self
                        .publish(identifier.clone(), receiver, stream_handler)
                        .await
                    {
                        Ok(()) => {
                            if let Some(notifier) = &self.notifier {
                                notifier.on_publish_notify(event_serialize_str).await;
                            }
                            self.streams_info
                                .insert(info.id, PubSubInfo::Publish { identifier });

                            Ok((frame_sender, packet_sender))
                        }
                        Err(err) => {
                            log::error!("event_loop Publish err: {}", err);
                            Err(err)
                        }
                    };

                    if result_sender.send(result).is_err() {
                        log::error!("event_loop Subscribe error: The receiver dropped.")
                    }
                }

                StreamHubEvent::UnPublish {
                    identifier,
                    info: _,
                } => {
                    if let Err(err) = self.unpublish(&identifier) {
                        log::error!(
                            "event_loop Unpublish err: {} with identifier: {}",
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
                    result_sender,
                } => {
                    let sub_id = info.id;
                    let info_clone = info.clone();

                    //new chan for Frame/Packet sender and receiver
                    let (sender, receiver) = match info.sub_data_type {
                        define::SubDataType::Frame => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                DataSender::Frame {
                                    sender: sender_chan,
                                },
                                DataReceiver {
                                    frame_receiver: Some(receiver_chan),
                                    packet_receiver: None,
                                },
                            )
                        }
                        define::SubDataType::Packet => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                DataSender::Packet {
                                    sender: sender_chan,
                                },
                                DataReceiver {
                                    frame_receiver: None,
                                    packet_receiver: Some(receiver_chan),
                                },
                            )
                        }
                    };

                    let rv = match self.subscribe(&identifier, info_clone, sender).await {
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
                            Ok(receiver)
                        }
                        Err(err) => {
                            log::error!("event_loop Subscribe error: {}", err);
                            Err(err)
                        }
                    };

                    if result_sender.send(rv).is_err() {
                        log::error!("event_loop Subscribe error: The receiver dropped.")
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
                        "event_loop ApiKickClient pub err: {} with identifier: {}",
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
                        "event_loop ApiKickClient pub err: {} with identifier: {}",
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
        sender: DataSender,
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
    pub async fn publish(
        &mut self,
        identifier: StreamIdentifier,
        receiver: DataReceiver,
        handler: Arc<dyn TStreamHandler>,
    ) -> Result<(), ChannelError> {
        if self.streams.get(&identifier).is_some() {
            return Err(ChannelError {
                value: ChannelErrorValue::Exists,
            });
        }

        let (event_publisher, event_consumer) = mpsc::unbounded_channel();
        let transmitter = Transmitter::new(receiver, event_consumer, handler);

        let identifier_clone = identifier.clone();

        if let Err(err) = transmitter.run().await {
            log::error!(
                "transmiter run error, idetifier: {}, error: {}",
                identifier_clone,
                err,
            );
        } else {
            log::info!("transmiter exits: idetifier: {}", identifier_clone);
        }

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
