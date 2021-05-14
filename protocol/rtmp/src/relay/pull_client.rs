use super::errors::PushClientError;
use super::errors::PushClientErrorValue;
use crate::channels::define::ChannelDataConsumer;
use crate::channels::define::ChannelEventProducer;
use crate::channels::define::ClientEvent;
use crate::channels::define::ClientEventConsumer;
use crate::session::client_session::ClientSession;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::session::client_session::ClientType;

use crate::channels::define::ChannelEvent;

pub struct PullClient {
    address: String,
    client_event_consumer: ClientEventConsumer,
    channel_event_producer: ChannelEventProducer,
}

impl PullClient {
    pub fn new(
        address: String,
        consumer: ClientEventConsumer,
        producer: ChannelEventProducer,
    ) -> Self {
        Self {
            address: address,

            client_event_consumer: consumer,
            channel_event_producer: producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), PushClientError> {
        loop {
            let val = self.client_event_consumer.recv().await?;
            match val {
                ClientEvent::Publish {
                    app_name,
                    stream_name,
                    connect_command_object,
                } => {}
            }
        }

        // let stream = TcpStream::connect(self.address.clone()).await?;

        // let mut client_session = ClientSession::new(
        //     stream,
        //     ClientType::Publish,
        //     app_name.clone(),
        //     stream_name.clone(),
        //     self.channel_event_producer.clone(),
        //     session_id,
        // );

        Ok(())
    }
}
