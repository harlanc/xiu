use {
    super::{
        define::{
            ChannelData, ChannelDataConsumer, ChannelDataPublisher, ChannelEvent,
            ChannelEventConsumer, ChannelEventPublisher, TransmitEvent, TransmitEventConsumer,
            TransmitEventPublisher,
        },
        errors::{ChannelError, ChannelErrorValue},
    },
    crate::cache::cache::Cache,
    std::{
        borrow::BorrowMut,
        cell::RefCell,
        collections::HashMap,
        sync::{Arc, Mutex},
    },
    tokio::sync::{broadcast, mpsc},
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
pub struct Transmiter {
    stream_consumer: ChannelDataConsumer, //used for publisher to produce AV data

    event_consumer: TransmitEventConsumer,
    event_producer: TransmitEventPublisher,

    player_producers: Arc<Mutex<Vec<ChannelDataPublisher>>>,
    cache: Arc<Mutex<Cache>>,
}

impl Transmiter {
    fn new(stream_consumer: ChannelDataConsumer) -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        Self {
            stream_consumer: stream_consumer,
            event_producer: event_producer,
            event_consumer: event_consumer,
            player_producers: Arc::new(Mutex::new(Vec::new())),

            cache: Arc::new(Mutex::new(Cache::new())),
        }
    }

    pub fn get_event_producer(&mut self) -> TransmitEventPublisher {
        return self.event_producer.clone();
    }

    // pub async fn run(&mut self) {
    //     tokio::spawn(async move {
    //         self.write_loop().await;
    //     });

    //     tokio::spawn(async move {
    //         self.subscriber_loop().await;
    //     });
    // val = self.stream_consumer.recv() => {
    //     match val{
    //         Ok(data)=>{

    //         }
    //         _ =>{}
    //     }
    // }
    // val = self.event_consumer.recv() => {
    //}

    pub async fn subscriber_loop(&mut self) {
        loop {
            let data = self.event_consumer.recv().await;
            match data {
                Some(val) => match val {
                    TransmitEvent::Subscribe { responder } => {
                        let (sender, receiver) = broadcast::channel(100);

                        responder.send(receiver);

                        let meta_body = self.cache.lock().unwrap().get_metadata();
                        let audio_seq = self.cache.lock().unwrap().get_audio_seq();
                        let video_seq = self.cache.lock().unwrap().get_video_seq();

                        sender.send(meta_body);
                        sender.send(audio_seq);
                        sender.send(video_seq);

                        let mut pro = self.player_producers.lock().unwrap();
                        pro.push(sender);
                    }
                },

                None => {}
            }
        }
    }

    pub async fn write_loop(&mut self) {
        loop {
            let data = self.stream_consumer.recv().await;
            match data {
                Ok(channel_data) => match channel_data {
                    ChannelData::MetaData { body } => {
                        self.cache.lock().unwrap().save_metadata(body);
                    }
                    ChannelData::Audio { timestamp, data } => {
                        let data = ChannelData::Audio {
                            timestamp: timestamp,
                            data: data.clone(),
                        };

                        for i in self.player_producers.lock().unwrap().iter() {
                            i.send(data.clone());
                        }
                    }
                    ChannelData::Video { timestamp, data } => {
                        let data = ChannelData::Video {
                            timestamp: timestamp,
                            data: data.clone(),
                        };
                        for i in self.player_producers.lock().unwrap().iter() {
                            i.send(data.clone());
                        }
                    }
                },
                Err(_) => {
                    return;
                }
            }
        }
    }
}

pub struct Channel {
    stream_producer: ChannelDataPublisher, //produce data from player to ChannelManager
    transmit_producer: ChannelDataPublisher, //transfer data from ChannelManager to Transmitter.
}

impl Channel {
    fn new(stream_producer: ChannelDataPublisher, transmit_producer: ChannelDataPublisher) -> Self {
        Self {
            stream_producer,
            transmit_producer,
        }
    }
}

pub struct ChannelsManager {
    //app_name to stream_name to producer
    channels: HashMap<String, HashMap<String, TransmitEventPublisher>>,
    //event is consumed in Channels, produced from other rtmp sessions
    event_consumer: ChannelEventConsumer,
    //event is produced from other rtmp sessions
    event_producer: ChannelEventPublisher,
}

impl ChannelsManager {
    pub fn new() -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();

        Self {
            channels: HashMap::new(),
            event_consumer,
            event_producer,
        }
    }
    pub async fn run(&mut self) {
        self.event_loop().await;
    }

    pub fn get_event_producer(&mut self) -> ChannelEventPublisher {
        return self.event_producer.clone();
    }

    pub async fn event_loop(&mut self) {
        while let Some(message) = self.event_consumer.recv().await {
            match message {
                ChannelEvent::Publish {
                    app_name,
                    stream_name,
                    responder,
                } => {
                    let rv = self.publish(&app_name, &stream_name);
                    match rv {
                        Ok(producer) => if let Err(_) = responder.send(producer) {},
                        Err(err) => continue,
                    }
                }
                ChannelEvent::UnPublish {
                    app_name,
                    stream_name,
                } => self.unpublish(&app_name, &stream_name),
                ChannelEvent::Subscribe {
                    app_name,
                    stream_name,
                    responder,
                } => {
                    let rv = self.subscribe(&app_name, &stream_name);
                    match rv {
                        Ok(consumer) => if let Err(_) = responder.send(consumer) {},
                        Err(err) => continue,
                    }
                }
                ChannelEvent::UnSubscribe {
                    app_name,
                    stream_name,
                } => {}
            }
        }
    }

    pub async fn data_loop(&mut self) {}

    //player subscribe a stream
    pub fn subscribe(
        &mut self,
        app_name: &String,
        stream_name: &String,
    ) -> Result<broadcast::Receiver<ChannelData>, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let (sender, receiver) = broadcast::channel(1);
                    producer.add_subscriber(sender);
                    return Ok(receiver);
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
    }

    //publish a stream
    pub fn publish(
        &mut self,
        app_name: &String,
        stream_name: &String,
    ) -> Result<ChannelDataPublisher, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get(stream_name) {
                Some(_) => {
                    return Err(ChannelError {
                        value: ChannelErrorValue::Exists,
                    })
                }
                None => {
                    let (player_sender, _) = broadcast::channel(100);
                    let (event_sender, _) = mpsc::unbounded_channel();
                    // let mut channel =
                    //     Channel::new(player_sender.clone(), chanmanager_sender.clone());
                    val.insert(stream_name.clone(), event_sender);

                    return Ok(sender);
                }
            },
            None => {
                let mut app = HashMap::new();
                let (sender, _) = broadcast::channel(100);
                let mut channel = Channel::new(sender.clone());
                tokio::spawn(async move {
                    channel.write_loop().await;
                });
                app.insert(stream_name.clone(), channel);
                self.channels.insert(app_name.clone(), app);
                return Ok(sender);
            }
        }
    }

    fn unpublish(&mut self, app_name: &String, stream_name: &String) {
        let rv = match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(_) => val.remove(stream_name),
                None => return,
            },
            None => return,
        };
    }

    //server broadcast data to player
    pub fn broadcast() {}
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
