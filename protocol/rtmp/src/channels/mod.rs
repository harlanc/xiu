//use super::statistics::SubscriberStatistics;

pub mod define;
pub mod errors;

use {
    crate::cache::Cache,
    crate::session::{common::SubscriberInfo, define::SubscribeType},
    define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventConsumer,
        ChannelEventProducer, ClientEvent, ClientEventConsumer, ClientEventProducer,
        TransmitterEvent, TransmitterEventConsumer, TransmitterEventProducer,
    },
    errors::{ChannelError, ChannelErrorValue},
    std::collections::HashMap,
    tokio::sync::{broadcast, mpsc, mpsc::UnboundedReceiver, oneshot},
    uuid::Uuid,
};

/************************************************************************************
* For a publisher, we new a broadcast::channel .
* For a player, we also new a oneshot::channel which subscribe the puslisher's broadcast channel,
* because we not only need to send av data from the publisher,but also some cache data(metadata
* and seq headers), so establishing a middle channel is needed.
************************************************************************************
*
*          stream_producer                      player_producers
*
*                                         sender(oneshot::channel) player
*                                    ----------------------------------
*                                   /     sender(oneshot::channel) player
*                                  /   --------------------------------
*           (broadcast::channel)  /   /   sender(oneshot::channel) player
* publisher --------------------->--------------------------------------
*                                 \   \   sender(oneshot::channel) player
*                                  \   --------------------------------
*                                   \     sender(oneshot::channel) player
*                                     ---------------------------------
*
*************************************************************************************/

//receive data from ChannelsManager and send to players
pub struct Transmitter {
    data_consumer: ChannelDataConsumer, //used for publisher to produce AV data
    event_consumer: TransmitterEventConsumer,
    subscriberid_to_producer: HashMap<Uuid, ChannelDataProducer>,
    cache: Cache,
}

impl Transmitter {
    fn new(
        app_name: String,
        stream_name: String,
        data_consumer: UnboundedReceiver<ChannelData>,
        event_consumer: UnboundedReceiver<TransmitterEvent>,
        gop_num: usize,
    ) -> Self {
        Self {
            data_consumer,
            event_consumer,
            subscriberid_to_producer: HashMap::new(),
            cache: Cache::new(app_name, stream_name, gop_num),
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError> {
        loop {
            tokio::select! {
                data = self.event_consumer.recv() =>{
                    if let Some(val) = data {
                        match val {
                            TransmitterEvent::Subscribe {
                                responder,
                                info,
                            } => {
                                let (sender, receiver) = mpsc::unbounded_channel();
                                responder.send(receiver).map_err(|_| ChannelError {
                                    value: ChannelErrorValue::SendError,
                                })?;

                                match info.sub_type {
                                    SubscribeType::PlayerRtmp
                                    | SubscribeType::PlayerHttpFlv
                                    | SubscribeType::PlayerHls => {
                                        if let Some(meta_body_data) = self.cache.get_metadata() {
                                            sender.send(meta_body_data).map_err(|_| ChannelError {
                                                value: ChannelErrorValue::SendError,
                                            })?;
                                        }
                                        if let Some(audio_seq_data) = self.cache.get_audio_seq() {
                                            sender.send(audio_seq_data).map_err(|_| ChannelError {
                                                value: ChannelErrorValue::SendError,
                                            })?;
                                        }
                                        if let Some(video_seq_data) = self.cache.get_video_seq() {
                                            sender.send(video_seq_data).map_err(|_| ChannelError {
                                                value: ChannelErrorValue::SendError,
                                            })?;
                                        }
                                        if let Some(gops_data) = self.cache.get_gops_data() {
                                            for gop in gops_data {
                                                for channel_data in gop.get_frame_data() {
                                                    sender.send(channel_data).map_err(|_| ChannelError {
                                                        value: ChannelErrorValue::SendError,
                                                    })?;
                                                }
                                            }
                                        }
                                    }
                                    SubscribeType::PublisherRtmp => {}
                                }
                                self.subscriberid_to_producer
                                    .insert(info.id, sender);
                            }
                            TransmitterEvent::UnSubscribe { info } => {
                                self.subscriberid_to_producer
                                    .remove(&info.id);
                            }
                            TransmitterEvent::UnPublish {} => {
                                return Ok(());
                            }
                        }
                    }
                }

                data = self.data_consumer.recv() =>{
                    if let Some(val) = data {
                        match val {
                            ChannelData::MetaData { timestamp, data } => {
                                self.cache.save_metadata(data, timestamp);
                            }
                            ChannelData::Audio { timestamp, data } => {
                                self.cache.save_audio_data(data.clone(), timestamp).await?;

                                let data = ChannelData::Audio {
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
                            ChannelData::Video { timestamp, data } => {
                                self.cache.save_video_data(data.clone(), timestamp).await?;

                                let data = ChannelData::Video {
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
                        }
                    }
                }

            }
        }

        //Ok(())
    }
}

pub struct ChannelsManager {
    //app_name to stream_name to producer
    channels: HashMap<String, HashMap<String, TransmitterEventProducer>>,
    //event is consumed in Channels, produced from other rtmp sessions
    channel_event_consumer: ChannelEventConsumer,
    //event is produced from other rtmp sessions
    channel_event_producer: ChannelEventProducer,
    //client_event_producer: client_event_producer
    client_event_producer: ClientEventProducer,
    rtmp_push_enabled: bool,
    rtmp_pull_enabled: bool,
    rtmp_gop_num: usize,
    hls_enabled: bool,
    // subscriber_statistics: HashMap<Uuid, SubscriberStatistics>,
}

impl Default for ChannelsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelsManager {
    pub fn new() -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        let (client_producer, _) = broadcast::channel(100);

        Self {
            channels: HashMap::new(),
            channel_event_consumer: event_consumer,
            channel_event_producer: event_producer,
            client_event_producer: client_producer,
            rtmp_push_enabled: false,
            rtmp_pull_enabled: false,
            rtmp_gop_num: 1,
            hls_enabled: false,
            //subscriber_statistics: HashMap::new(),
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

    pub fn set_rtmp_gop_num(&mut self, gop_num: usize) {
        self.rtmp_gop_num = gop_num;
    }

    pub fn set_hls_enabled(&mut self, enabled: bool) {
        self.hls_enabled = enabled;
    }

    pub fn get_session_event_producer(&mut self) -> ChannelEventProducer {
        self.channel_event_producer.clone()
    }

    pub fn get_client_event_consumer(&mut self) -> ClientEventConsumer {
        self.client_event_producer.subscribe()
    }

    pub async fn event_loop(&mut self) {
        while let Some(message) = self.channel_event_consumer.recv().await {
            log::info!("{}", message);
            match message {
                ChannelEvent::Publish {
                    app_name,
                    stream_name,
                    responder,
                } => {
                    let rv = self.publish(&app_name, &stream_name);
                    match rv {
                        Ok(producer) => {
                            if responder.send(producer).is_err() {
                                log::error!("event_loop responder send err");
                            }
                        }
                        Err(err) => {
                            log::error!("event_loop Publish err: {}\n", err);
                            continue;
                        }
                    }
                }

                ChannelEvent::UnPublish {
                    app_name,
                    stream_name,
                } => {
                    if let Err(err) = self.unpublish(&app_name, &stream_name) {
                        log::error!(
                            "event_loop Unpublish err: {} with app name: {} stream name :{}\n",
                            err,
                            app_name,
                            stream_name
                        );
                    }
                }
                ChannelEvent::Subscribe {
                    app_name,
                    stream_name,
                    info,
                    responder,
                } => {
                    let rv = self.subscribe(&app_name, &stream_name, info).await;
                    match rv {
                        Ok(consumer) => {
                            if responder.send(consumer).is_err() {
                                log::error!("event_loop Subscribe err");
                            }
                        }
                        Err(err) => {
                            log::error!("event_loop Subscribe error: {}", err);
                            continue;
                        }
                    }
                }
                ChannelEvent::UnSubscribe {
                    app_name,
                    stream_name,
                    info,
                } => {
                    let _ = self.unsubscribe(&app_name, &stream_name, info);
                }
            }
        }
    }

    //player subscribe a stream
    pub async fn subscribe(
        &mut self,
        app_name: &String,
        stream_name: &String,
        sub_info: SubscriberInfo,
    ) -> Result<mpsc::UnboundedReceiver<ChannelData>, ChannelError> {
        if let Some(val) = self.channels.get_mut(app_name) {
            if let Some(producer) = val.get_mut(stream_name) {
                let (sender, receiver) = oneshot::channel();

                let event = TransmitterEvent::Subscribe {
                    responder: sender,
                    info: sub_info,
                };

                producer.send(event).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;

                if let Ok(consumer) = receiver.await {
                    log::info!(
                        "subscribe get consumer successfully, app_name: {}, stream_name: {}",
                        app_name,
                        stream_name
                    );
                    return Ok(consumer);
                }
            }
        }

        if self.rtmp_pull_enabled {
            log::info!(
                "subscribe: try to pull stream, app_name: {}, stream_name: {}",
                app_name,
                stream_name
            );

            let client_event = ClientEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
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
        app_name: &String,
        stream_name: &String,
        sub_info: SubscriberInfo,
    ) -> Result<(), ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitterEvent::UnSubscribe { info: sub_info };
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;
                }
                None => {
                    return Err(ChannelError {
                        value: ChannelErrorValue::NoStreamName,
                    })
                }
            },
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
        app_name: &String,
        stream_name: &String,
    ) -> Result<ChannelDataProducer, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => {
                if val.get(stream_name).is_some() {
                    return Err(ChannelError {
                        value: ChannelErrorValue::Exists,
                    });
                }
            }
            None => {
                let stream_map = HashMap::new();
                self.channels.insert(app_name.clone(), stream_map);
            }
        }

        if let Some(stream_map) = self.channels.get_mut(app_name) {
            let (event_publisher, event_consumer) = mpsc::unbounded_channel();
            let (data_publisher, data_consumer) = mpsc::unbounded_channel();

            let mut transmitter = Transmitter::new(
                app_name.clone(),
                stream_name.clone(),
                data_consumer,
                event_consumer,
                self.rtmp_gop_num,
            );

            let app_name_clone = app_name.clone();
            let stream_name_clone = stream_name.clone();

            tokio::spawn(async move {
                if let Err(err) = transmitter.run().await {
                    log::error!(
                        "transmiter run error, app_name: {}, stream_name: {}, error: {}",
                        app_name_clone,
                        stream_name_clone,
                        err,
                    );
                } else {
                    log::info!(
                        "transmiter exists: app_name: {}, stream_name: {}",
                        app_name_clone,
                        stream_name_clone
                    );
                }
            });

            stream_map.insert(stream_name.clone(), event_publisher);

            if self.rtmp_push_enabled || self.hls_enabled {
                let client_event = ClientEvent::Publish {
                    app_name: app_name.clone(),
                    stream_name: stream_name.clone(),
                };

                //send publish info to push clients
                self.client_event_producer
                    .send(client_event)
                    .map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;
            }

            Ok(data_publisher)
        } else {
            Err(ChannelError {
                value: ChannelErrorValue::NoAppName,
            })
        }
    }

    fn unpublish(&mut self, app_name: &String, stream_name: &String) -> Result<(), ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitterEvent::UnPublish {};
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;
                    val.remove(stream_name);
                    log::info!(
                        "unpublish remove stream, app_name: {},stream_name: {}",
                        app_name,
                        stream_name
                    );
                }
                None => {
                    return Err(ChannelError {
                        value: ChannelErrorValue::NoStreamName,
                    })
                }
            },
            None => {
                return Err(ChannelError {
                    value: ChannelErrorValue::NoAppName,
                })
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::cell::RefCell;

    use std::sync::Arc;
    pub struct TestFunc {}

    impl TestFunc {
        fn new() -> Self {
            Self {}
        }
        pub fn aaa(&mut self) {}
    }

    //https://juejin.cn/post/6844904105698148360
    #[test]
    fn test_lock() {
        let channel = Arc::new(RefCell::new(TestFunc::new()));
        channel.borrow_mut().aaa();
    }
}
