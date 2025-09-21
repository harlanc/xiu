use {
    super::{errors::HlsError, flv_data_receiver::FlvDataReceiver},
    config::HlsConfig,
    streamhub::{
        define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
        stream::StreamIdentifier,
    },
};

pub struct HlsRemuxer {
    client_event_consumer: BroadcastEventReceiver,
    event_producer: StreamHubEventSender,
    hls_config: Option<HlsConfig>,
}

impl HlsRemuxer {
    pub fn new(
        consumer: BroadcastEventReceiver,
        event_producer: StreamHubEventSender,
        hls_config: Option<HlsConfig>
    ) -> Self {
        Self {
            client_event_consumer: consumer,
            event_producer,
            hls_config,
        }
    }

    pub async fn run(&mut self) -> Result<(), HlsError> {
        loop {
            let val = self.client_event_consumer.recv().await?;
            match val {
                BroadcastEvent::Publish { identifier } => {
                    if let StreamIdentifier::Rtmp {
                        app_name,
                        stream_name,
                    } = identifier
                    {
                        let mut rtmp_subscriber = FlvDataReceiver::new(
                            app_name,
                            stream_name,
                            self.event_producer.clone(),
                            self.hls_config.clone(),
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
