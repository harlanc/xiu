use {
    super::{errors::HlsError, flv_data_receiver::FlvDataReceiver},
    streamhub::{
        define::{StreamHubEventSender, ClientEvent, ClientEventConsumer},
        stream::StreamIdentifier,
    },
};

pub struct RtmpEventProcessor {
    client_event_consumer: ClientEventConsumer,
    event_producer: StreamHubEventSender,
}

impl RtmpEventProcessor {
    pub fn new(consumer: ClientEventConsumer, event_producer: StreamHubEventSender) -> Self {
        Self {
            client_event_consumer: consumer,
            event_producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), HlsError> {
        loop {
            let val = self.client_event_consumer.recv().await?;
            match val {
                ClientEvent::Publish { identifier } => {
                    if let StreamIdentifier::Rtmp {
                        app_name,
                        stream_name,
                    } = identifier
                    {
                        let mut rtmp_subscriber = FlvDataReceiver::new(
                            app_name,
                            stream_name,
                            self.event_producer.clone(),
                            5,
                        );

                        tokio::spawn(async move {
                            if let Err(err) = rtmp_subscriber.run().await {
                                println!("hls handler run error {err}");
                            }
                        });
                    }
                }
                _ => {
                    log::trace!("other infos...");
                }
            }
        }
    }
}
