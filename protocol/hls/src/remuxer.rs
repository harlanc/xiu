use {
    super::{errors::HlsError, flv_data_receiver::FlvDataReceiver},
    streamhub::{
        define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
        stream::StreamIdentifier,
    },
};

pub struct HlsRemuxer {
    client_event_consumer: BroadcastEventReceiver,
    event_producer: StreamHubEventSender,
    need_record: bool,
    path: String,
    fragment: i64,
    aof_ratio: i64,
}

impl HlsRemuxer {
    pub fn new(
        consumer: BroadcastEventReceiver,
        event_producer: StreamHubEventSender,
        need_record: bool,
        path: Option<String>,
        fragment: Option<i64>,
        aof_ratio: Option<i64>,
    ) -> Self {
        Self {
            client_event_consumer: consumer,
            event_producer,
            need_record,
            path: path.unwrap_or("./".to_string()),
            fragment: fragment.unwrap_or(2),
            aof_ratio: aof_ratio.unwrap_or(1),
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
                            self.fragment,
                            self.need_record,
                            self.path.clone(),
                            self.aof_ratio,
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
