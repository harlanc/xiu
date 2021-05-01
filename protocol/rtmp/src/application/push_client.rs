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
pub struct PushClient {
    address: String,
    client_event_consumer: ClientEventConsumer,
    channel_event_producer: ChannelEventProducer,

    data_consumer: ChannelDataConsumer,
}

impl PushClient {
    pub fn new(
        address: String,
        consumer: ClientEventConsumer,
        producer: ChannelEventProducer,
    ) -> Self {
        let (_, init_consumer) = mpsc::unbounded_channel();

        Self {
            client_event_consumer: consumer,
            channel_event_producer: producer,
            data_consumer: init_consumer,
            address: address,
        }
    }

    pub async fn run(&mut self) -> Result<(), PushClientError> {
        let mut session_id = std::u64::MAX;
        loop {
            match self.client_event_consumer.recv().await? {
                ClientEvent::Publish {
                    app_name,
                    stream_name,
                } => {
                    let stream = TcpStream::connect(self.address.clone()).await?;

                    let mut client_session = ClientSession::new(
                        stream,
                        ClientType::Publish,
                        app_name.clone(),
                        stream_name.clone(),
                        self.channel_event_producer.clone(),
                        session_id,
                    );

                    tokio::spawn(async move {
                        if let Err(err) = client_session.run().await {
                            print!(" session error {}\n", err);
                        }
                    });

                    session_id = session_id - 1;
                }

                _ => {}
            }
        }
    }
}
