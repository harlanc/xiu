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

    player_producers: Arc<Mutex<HashMap<u64, ChannelDataPublisher>>>,
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

    pub async fn run(&mut self) -> Result<(), ChannelError> {
        loop {
            tokio::select! {
                data = self.event_consumer.recv() =>{
                    if let Some(val) = data{
                        print!("receive player event\n");
                        match val{
                            TransmitEvent::Subscribe { responder,session_id } => {
                                let ( sender, receiver) = mpsc::unbounded_channel();

                                responder.send(receiver).map_err(|_| ChannelError {
                                    value: ChannelErrorValue::SendError,
                                })?;

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

                                let mut pro = self.player_producers.lock().unwrap();
                                pro.insert(session_id,sender);

                            },
                            TransmitEvent::UnSubscribe{session_id} =>{

                                let mut pro = self.player_producers.lock().unwrap();
                                pro.remove(&session_id);

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
                                        value: ChannelErrorValue::SendError,
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
                                        value: ChannelErrorValue::SendError,
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
                        Err(_) => continue,
                    }
                }

                ChannelEvent::UnPublish {
                    app_name,
                    stream_name,
                } => {
                    let _ = self.unpublish(&app_name, &stream_name);
                }
                ChannelEvent::Subscribe {
                    app_name,
                    stream_name,
                    session_id,
                    responder,
                } => {
                    let rv = self.subscribe(&app_name, &stream_name, session_id).await;
                    match rv {
                        Ok(consumer) => if let Err(_) = responder.send(consumer) {},
                        Err(_) => continue,
                    }
                }
                ChannelEvent::UnSubscribe {
                    app_name,
                    stream_name,
                    session_id,
                } => {
                    let _ = self.unsubscribe(&app_name, &stream_name, session_id);
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
        session_id: u64,
    ) -> Result<mpsc::UnboundedReceiver<ChannelData>, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let (sender, receiver) = oneshot::channel();

                    let event = TransmitEvent::Subscribe {
                        responder: sender,
                        session_id,
                    };
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;

                    match receiver.await {
                        Ok(consumer) => {
                            return Ok(consumer);
                        }
                        Err(_) => {
                            return Err(ChannelError {
                                value: ChannelErrorValue::NoStreamName,
                            });
                        }
                    }
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

    pub fn unsubscribe(
        &mut self,
        app_name: &String,
        stream_name: &String,
        session_id: u64,
    ) -> Result<(), ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitEvent::UnSubscribe { session_id };
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
    ) -> Result<ChannelDataPublisher, ChannelError> {
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
                let _ = transmiter.run().await;
            });

            stream_map.insert(stream_name.clone(), event_publisher);

            return Ok(data_publisher);
        } else {
            return Err(ChannelError {
                value: ChannelErrorValue::NoAppName,
            });
        }
    }

    fn unpublish(&mut self, app_name: &String, stream_name: &String) -> Result<(), ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(producer) => {
                    let event = TransmitEvent::UnPublish {};
                    producer.send(event).map_err(|_| ChannelError {
                        value: ChannelErrorValue::SendError,
                    })?;
                    val.remove(stream_name);
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
