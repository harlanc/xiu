use super::errors::PushError;
use super::errors::PushErrorValue;
use crate::channels::define::ChannelDataConsumer;
use crate::channels::define::ChannelEventProducer;
use crate::channels::define::ClientEvent;
use crate::channels::define::ClientEventConsumer;
use crate::session::client_session::ClientSession;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::channels::define::ChannelEvent;
pub struct PushClient {
    client_event_consumer: ClientEventConsumer,
    channel_event_producer: ChannelEventProducer,

    data_consumer: ChannelDataConsumer,
}

impl PushClient {
    pub fn new(consumer: ClientEventConsumer, producer: ChannelEventProducer) -> Self {
        let (_, init_consumer) = mpsc::unbounded_channel();

        Self {
            client_event_consumer: consumer,
            channel_event_producer: producer,
            data_consumer: init_consumer,
        }
    }

    pub async fn start(&mut self) -> Result<(), PushError> {
        loop {
            match self.client_event_consumer.recv().await? {
                ClientEvent::Publish {
                    app_name,
                    stream_name,
                } => {
                    self.subscribe_from_channels(app_name, stream_name).await?;

                    let client_session = ClientSession::new(stream, client_type, app_name, stream_name)
                }

                _ => {}
            }
        }
    }

    async fn subscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
    ) -> Result<(), PushError> {
        let (sender, receiver) = oneshot::channel();
        let subscribe_event = ChannelEvent::Subscribe {
            app_name: app_name,
            stream_name,
            session_id: 100,
            responder: sender,
        };

        let rv = self.channel_event_producer.send(subscribe_event);
        match rv {
            Err(_) => {
                return Err(PushError {
                    value: PushErrorValue::SendError,
                })
            }
            _ => {}
        }

        match receiver.await {
            Ok(consumer) => {
                self.data_consumer = consumer;
            }
            Err(_) => {}
        }
        Ok(())
    }
}
