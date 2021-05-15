use {
    super::errors::ClientError,
    crate::{
        channels::define::{ChannelEventProducer, ClientEvent, ClientEventConsumer},
        session::client_session::{ClientSession, ClientType},
    },
    tokio::net::TcpStream,
};

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

    pub async fn run(&mut self) -> Result<(), ClientError> {
        let mut session_id = std::u64::MAX;

        loop {
            let val = self.client_event_consumer.recv().await?;
            match val {
                ClientEvent::Subscribe {
                    app_name,
                    stream_name,
                } => {
                    let stream = TcpStream::connect(self.address.clone()).await?;

                    let mut client_session = ClientSession::new(
                        stream,
                        ClientType::Play,
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
