use streamhub::stream::StreamIdentifier;

use {
    super::errors::ClientError,
    crate::session::client_session::{ClientSession, ClientType},
    streamhub::define::{StreamHubEventSender, ClientEvent, ClientEventConsumer},
    tokio::net::TcpStream,
};

pub struct PullClient {
    address: String,
    client_event_consumer: ClientEventConsumer,
    channel_event_producer: StreamHubEventSender,
}

impl PullClient {
    pub fn new(
        address: String,
        consumer: ClientEventConsumer,
        producer: StreamHubEventSender,
    ) -> Self {
        Self {
            address,

            client_event_consumer: consumer,
            channel_event_producer: producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        loop {
            let val = self.client_event_consumer.recv().await?;

            if let ClientEvent::Subscribe { identifier } = val {
                if let StreamIdentifier::Rtmp {
                    app_name,
                    stream_name,
                } = identifier
                {
                    log::info!(
                        "receive pull event, app_name :{}, stream_name: {}",
                        app_name,
                        stream_name
                    );
                    let stream = TcpStream::connect(self.address.clone()).await?;

                    let mut client_session = ClientSession::new(
                        stream,
                        ClientType::Play,
                        self.address.clone(),
                        app_name.clone(),
                        stream_name.clone(),
                        self.channel_event_producer.clone(),
                        0,
                    );

                    tokio::spawn(async move {
                        if let Err(err) = client_session.run().await {
                            log::error!("client_session as pull client run error: {}", err);
                        }
                    });
                }
            }
        }
    }
}
