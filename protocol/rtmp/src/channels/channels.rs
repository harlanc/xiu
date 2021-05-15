use tokio::sync::broadcast;

use {
    super::{
        define::{
            ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent,
            ChannelEventConsumer, ChannelEventProducer, ClientEvent, ClientEventConsumer,
            ClientEventProducer, TransmitEvent, TransmitEventConsumer, TransmitEventPublisher,
        },
        errors::{ChannelError, ChannelErrorValue},
    },
    crate::cache::cache::Cache,
    crate::session::{common::SessionInfo, define::SessionSubType},
    std::{
        //borrow::BorrowMut,
        //cell::RefCell,
        collections::HashMap,
        sync::{Arc, Mutex},
    },
    tokio::sync::{mpsc, mpsc::UnboundedReceiver, oneshot},
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
    data_consumer: ChannelDataConsumer, //used for publisher to produce AV data
    event_consumer: TransmitEventConsumer,

    player_producers: Arc<Mutex<HashMap<u64, ChannelDataProducer>>>,
    cache: Arc<Mutex<Cache>>,
}

impl Transmiter {
    fn new(
        data_consumer: UnboundedReceiver<ChannelData>,
        event_consumer: UnboundedReceiver<TransmitEvent>,
    ) -> Self {
        Self {
            data_consumer: data_consumer,
            event_consumer: event_consumer,
            player_producers: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(Mutex::new(Cache::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError> {
        loop {
            tokio::select! {
                data = self.event_consumer.recv() =>{
                    if let Some(val) = data{
                        print!("receive player event\n");
                        match val{
                            TransmitEvent::Subscribe { responder,session_info } => {

                                let ( sender, receiver) = mpsc::unbounded_channel();
                                responder.send(receiver).map_err(|_| ChannelError {
                                    value: ChannelErrorValue::SendError,
                                })?;

                                match session_info.session_sub_type {
                                    SessionSubType::Player=>{
                                        let meta_body = self.cache.lock().unwrap().get_metadata();
                                        let audio_seq = self.cache.lock().unwrap().get_audio_seq();
                                        let video_seq = self.cache.lock().unwrap().get_video_seq();

                                        sender.send(meta_body).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendError,
                                        })?;
                                        sender.send(audio_seq).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendError,
                                        })?;
                                        sender.send(video_seq).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendError,
                                        })?;

                                    }
                                    SessionSubType::Publisher =>{

                                    }

                                }


                                let mut pro = self.player_producers.lock().unwrap();
                                pro.insert(session_info.session_id, sender);

                            },
                            TransmitEvent::UnSubscribe{session_info} =>{

                                let mut pro = self.player_producers.lock().unwrap();
                                pro.remove(&session_info.session_id);

                            },
                            TransmitEvent::UnPublish{} => {
                                return Ok(());
                            },


                        }

                    }
                }

                data = self.data_consumer.recv() =>{

                    if let Some(val) = data{

                        match val {
                            ChannelData::MetaData { body } => {
                                self.cache.lock().unwrap().save_metadata(body);
                            }
                            ChannelData::Audio { timestamp, data } => {

                                self.cache.lock().unwrap().save_audio_seq(data.clone(),timestamp)?;

                                let data = ChannelData::Audio {
                                    timestamp: timestamp,
                                    data: data.clone(),
                                };


                                for (_,v) in self.player_producers.lock().unwrap().iter() {
                                    v.send(data.clone()).map_err(|_| ChannelError {
                                            value: ChannelErrorValue::SendAudioError,
                                    })?;
                                }
                            }
                            ChannelData::Video { timestamp, data } => {

                                self.cache.lock().unwrap().save_video_seq(data.clone(),timestamp)?;

                                let data = ChannelData::Video {
                                    timestamp: timestamp,
                                    data: data.clone(),
                                };
                                for (_,v) in self.player_producers.lock().unwrap().iter() {
                                    v.send(data.clone()).map_err(|_| ChannelError {
                                        value: ChannelErrorValue::SendVideoError,
                                    })?;
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
    channels: HashMap<String, HashMap<String, TransmitEventPublisher>>,
    //event is consumed in Channels, produced from other rtmp sessions
    channel_event_consumer: ChannelEventConsumer,
    //event is produced from other rtmp sessions
    channel_event_producer: ChannelEventProducer,
    //client_event_producer: client_event_producer
    client_event_producer: ClientEventProducer,
    push_enabled: bool,
    pull_enabled: bool,
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
            push_enabled: false,
            pull_enabled: false,
        }
    }
    pub async fn run(&mut self) {
        self.event_loop().await;
    }

    pub fn set_push_enabled(&mut self, enabled: bool) {
        self.push_enabled = enabled;
    }

    pub fn set_pull_enabled(&mut self, enabled: bool) {
        self.pull_enabled = enabled;
    }

    pub fn get_session_event_producer(&mut self) -> ChannelEventProducer {
        return self.channel_event_producer.clone();
    }

    pub fn get_client_event_consumer(&mut self) -> ClientEventConsumer {
        return self.client_event_producer.subscribe();
    }

    pub async fn event_loop(&mut self) {
        while let Some(message) = self.channel_event_consumer.recv().await {
            match message {
                ChannelEvent::Publish {
                    app_name,
                    stream_name,
                    responder,
                } => {
                    let rv = self.publish(&app_name, &stream_name);
                    match rv {
                        Ok(producer) => {
                            if let Err(_) = responder.send(producer) {
                                print!("event_loop responder send err\n");
                            }
                        }
                        Err(err) => {
                            print!("event_loop Publish err: {}\n", err);
                            continue;
                        }
                    }
                }

                ChannelEvent::UnPublish {
                    app_name,
                    stream_name,
                } => {
                    if let Err(err) = self.unpublish(&app_name, &stream_name) {
                        println!("unpublish err: {}", err);
                    }
                }
                ChannelEvent::Subscribe {
                    app_name,
                    stream_name,
                    session_info,
                    responder,
                } => {
                    let rv = self.subscribe(&app_name, &stream_name, session_info).await;
                    match rv {
                        Ok(consumer) => if let Err(_) = responder.send(consumer) {},
                        Err(_) => continue,
                    }
                }
                ChannelEvent::UnSubscribe {
                    app_name,
                    stream_name,
                    session_info,
                } => {
                    let _ = self.unsubscribe(&app_name, &stream_name, session_info);
                }
            }
        }
    }

    pub async fn data_loop(&mut self) {}

    //player subscribe a stream
    pub async fn subscribe(
        &mut self,
        app_name: &String,
        stream_name: &String,
        session_info: SessionInfo,
    ) -> Result<mpsc::UnboundedReceiver<ChannelData>, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let (sender, receiver) = oneshot::channel();

                    let event = TransmitEvent::Subscribe {
                        responder: sender,
                        session_info,
                    };
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;

                    match receiver.await {
                        Ok(consumer) => {
                            return Ok(consumer);
                        }
                        Err(_) => {
                            // return Err(ChannelError {
                            //     value: ChannelErrorValue::NoStreamName,
                            // });
                        }
                    }
                }
                None => {
                    // return Err(ChannelError {
                    //     value: ChannelErrorValue::NoStreamName,
                    // })
                }
            },
            None => {

                // return Err(ChannelError {
                //     value: ChannelErrorValue::NoAppName,
                // })
            }
        }

        if self.pull_enabled {
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

        return Err(ChannelError {
            value: ChannelErrorValue::NoAppOrStreamName,
        });
    }

    pub fn unsubscribe(
        &mut self,
        app_name: &String,
        stream_name: &String,
        session_info: SessionInfo,
    ) -> Result<(), ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitEvent::UnSubscribe { session_info };
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
            Some(val) => match val.get(stream_name) {
                Some(_) => {
                    return Err(ChannelError {
                        value: ChannelErrorValue::Exists,
                    })
                }
                None => {}
            },
            None => {
                let stream_map = HashMap::new();
                self.channels.insert(app_name.clone(), stream_map);
            }
        }

        if let Some(stream_map) = self.channels.get_mut(app_name) {
            let (event_publisher, event_consumer) = mpsc::unbounded_channel();
            let (data_publisher, data_consumer) = mpsc::unbounded_channel();

            let mut transmiter = Transmiter::new(data_consumer, event_consumer);
            tokio::spawn(async move {
                if let Err(err) = transmiter.run().await {
                    print!("transmiter error {}\n", err);
                }
            });

            stream_map.insert(stream_name.clone(), event_publisher);

            if self.push_enabled {
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

            return Ok(data_publisher);
        } else {
            return Err(ChannelError {
                value: ChannelErrorValue::NoAppName,
            });
        }
    }

    fn unpublish(&mut self, app_name: &String, stream_name: &String) -> Result<(), ChannelError> {
        println!("unpublish begin...{} {}", app_name, stream_name);
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitEvent::UnPublish {};
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;
                    val.remove(stream_name);
                    print!("remove stream name{}\n", stream_name);
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
