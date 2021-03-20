use crate::{chunk, messages};

use super::define::ChannelEvent;
use super::define::MultiConsumerForData;
use super::define::MultiProducerForEvent;
use super::define::SingleConsumerForEvent;
use super::define::SingleProducerForData;
use super::errors::ChannelError;
use super::errors::ChannelErrorValue;
use std::sync::Arc;
use std::{borrow::BorrowMut, collections::HashMap};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub struct Channels {
    streams: HashMap<String, HashMap<String, SingleProducerForData>>,
    //event is consumed in Channels, produced from other rtmp sessions
    event_consumer: SingleConsumerForEvent,
    //event is produced from other rtmp sessions
    event_producer: MultiProducerForEvent,
    //rtmp data is consumed by other rtmp sessions
    //data_consumer: MultiConsumerForData,
    // //rtmp data is produced by a rtmp session
    // data_producer: SingleProducerForData,
}

impl Channels {
    pub fn new() -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        //let (data_producer, data_consumer) = broadcast::channel(100);

        Self {
            streams: HashMap::new(),
            event_consumer,
            event_producer,
            //data_consumer,
            // data_producer,
        }
    }
    pub async fn run(&mut self) {
        self.event_loop().await;
    }

    pub fn get_event_producer(&mut self) -> MultiProducerForEvent {
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
    ) -> Result<MultiConsumerForData, ChannelError> {
        match self.streams.get(app_name) {
            Some(val) => match val.get(stream_name) {
                Some(producer) => return Ok(producer.subscribe()),
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
    ) -> Result<SingleProducerForData, ChannelError> {
        match self.streams.get_mut(app_name) {
            Some(val) => match val.get(stream_name) {
                Some(_) => {
                    return Err(ChannelError {
                        value: ChannelErrorValue::Exists,
                    })
                }
                None => {
                    let (sender, _) = broadcast::channel(100);
                    val.insert(stream_name.clone(), sender.clone());
                    return Ok(sender);
                }
            },
            None => {
                let mut app = HashMap::new();
                let (sender, _) = broadcast::channel(100);
                app.insert(stream_name.clone(), sender.clone());
                self.streams.insert(app_name.clone(), app);
                return Ok(sender);
            }
        }
    }

    fn unpublish(&mut self, app_name: &String, stream_name: &String) {
        let rv = match self.streams.get_mut(app_name) {
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
