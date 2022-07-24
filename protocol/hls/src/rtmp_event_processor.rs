use super::errors::HlsError;
use super::flv_data_receiver::FlvDataReceiver;
use super::hls_event_manager::{DispatchEvent, DispatchEventProducer};
use crate::hls_event_manager::HlsEventProducer;
use rtmp::channels::define::ChannelEventProducer;
use rtmp::channels::define::ClientEvent;
use rtmp::channels::define::ClientEventConsumer;
use tokio::sync::oneshot;

pub struct RtmpEventProcessor {
    client_event_consumer: ClientEventConsumer,
    event_producer: ChannelEventProducer,
    hls_manager_dispatcher: DispatchEventProducer,
}

impl RtmpEventProcessor {
    pub fn new(
        consumer: ClientEventConsumer,
        event_producer: ChannelEventProducer,
        hls_manager_dispatcher: DispatchEventProducer,
    ) -> Self {
        Self {
            client_event_consumer: consumer,
            event_producer,
            hls_manager_dispatcher,
        }
    }

    pub async fn run(&mut self) -> Result<(), HlsError> {
        loop {
            let val = self.client_event_consumer.recv().await?;
            match val {
                ClientEvent::Publish {
                    app_name,
                    stream_name,
                } => {
                    let (resp_tx, resp_rx) = oneshot::channel();

                    let m = DispatchEvent::CreateChannel {
                        stream_name: stream_name.clone(),
                        channel: resp_tx,
                    };

                    self.hls_manager_dispatcher.send(m).await;

                    let (stream_channel_producer, m3u8_consumer) = resp_rx.await.unwrap();

                    let mut rtmp_subscriber = FlvDataReceiver::new(
                        app_name,
                        stream_name,
                        self.event_producer.clone(),
                        stream_channel_producer,
                        m3u8_consumer,
                        5,
                    );

                    tokio::spawn(async move {
                        if let Err(err) = rtmp_subscriber.run().await {
                            print!("hls handler run error {}\n", err);
                        }
                    });
                }
                _ => {
                    log::trace!("other infos...");
                }
            }
        }
    }
}
