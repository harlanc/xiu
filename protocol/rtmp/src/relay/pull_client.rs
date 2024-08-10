use streamhub::stream::StreamIdentifier;

use {
    super::errors::ClientError,
    crate::session::client_session::{ClientSession, ClientSessionType},
    streamhub::define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
    tokio::net::TcpStream,
};

pub struct PullClient {
    address: String,
    client_event_consumer: BroadcastEventReceiver,
    channel_event_producer: StreamHubEventSender,
}

impl PullClient {
    pub fn new(
        address: String,
        consumer: BroadcastEventReceiver,
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
            let event = self.client_event_consumer.recv().await?;

            if let BroadcastEvent::Subscribe {
                id: _,
                identifier:
                    StreamIdentifier::Rtmp {
                        app_name,
                        stream_name,
                    },
                server_address: _,
                result_sender: _,
            } = event
            {
                log::info!(
                    "receive pull event, app_name :{}, stream_name: {}",
                    app_name,
                    stream_name
                );
                let stream = TcpStream::connect(self.address.clone()).await?;

                let mut client_session = ClientSession::new(
                    stream,
                    ClientSessionType::Pull,
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
