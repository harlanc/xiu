use crate::{chunk, messages};

use super::define::ChannelDataConsumer;
use super::define::ChannelDataPublisher;
use super::define::ChannelEvent;
use super::define::ChannelEventConsumer;
use super::define::ChannelEventPublisher;
use super::define::PlayerConsumer;
use super::define::{ChannelData, PlayerPublisher};

use super::errors::ChannelError;
use super::errors::ChannelErrorValue;
use std::sync::Arc;
use std::{borrow::Borrow, borrow::BorrowMut, collections::HashMap};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
use std::cell::RefCell;

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

pub struct Channel {
    stream_producer: ChannelDataPublisher, //used for publisher to produce AV data
    player_producers: RefCell<Vec<PlayerPublisher>>, // consumers who subscribe this channel.
}

impl Channel {
    fn new(producer: ChannelDataPublisher) -> Self {
        Self {
            stream_producer: producer,
            player_producers: RefCell::new(Vec::new()),
        }
    }

    fn add_subscriber(&mut self, producer: PlayerPublisher) {
        self.player_producers.borrow_mut().push(producer);
    }

    async fn run(&mut self) {
        let mut sub = self.stream_producer.subscribe();
        loop {
            let data = sub.recv().await;
            match data {
                Ok(d) => {
                    let producers = self.player_producers.borrow_mut();
                    for i in 0..producers.len(){

                        producers[i].send(d);

                    }
                }
                Err(_) => {}
            }
        }
    }
}
pub struct ChannelsManager {
    //app_name to stream_name to producer
    channels: HashMap<String, HashMap<String, Channel>>,
    //event is consumed in Channels, produced from other rtmp sessions
    event_consumer: ChannelEventConsumer,
    //event is produced from other rtmp sessions
    event_producer: ChannelEventPublisher,
    //rtmp data is consumed by other rtmp sessions
    //data_consumer: MultiConsumerForData,
    // //rtmp data is produced by a rtmp session
    // data_producer: SingleProducerForData,
}

impl ChannelsManager {
    pub fn new() -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        //let (data_producer, data_consumer) = broadcast::channel(100);

        Self {
            channels: HashMap::new(),
            event_consumer,
            event_producer,
            //data_consumer,
            // data_producer,
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
    ) -> Result<oneshot::Receiver<ChannelData>, ChannelError> {
        match self.channels.get_mut(app_name) {
            Some(val) => match val.get_mut(stream_name) {
                Some(mut producer) => {
                    let (sender, receiver) = oneshot::channel();

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
                    let (sender, _) = broadcast::channel(100);
                    let channel = Channel::new(sender.clone());
                    val.insert(stream_name.clone(), channel);
                    return Ok(sender);
                }
            },
            None => {
                let mut app = HashMap::new();
                let (sender, _) = broadcast::channel(100);
                let channel = Channel::new(sender.clone());
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
