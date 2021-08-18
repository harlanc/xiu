use {
    super::{
        errors::{HlsError, HlsErrorValue},
        flv2hls::Flv2HlsRemuxer,
    },
    rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelEvent, ChannelEventProducer,
    },
    rtmp::session::{
        common::SessionInfo,
        define::SessionSubType,
        errors::{SessionError, SessionErrorValue},
    },
    std::time::Duration,
    tokio::{
        sync::{mpsc, oneshot},
        time::sleep,
    },
    uuid::Uuid,
    xflv::define::FlvData,
};

// use super::errors::HlsError;
// use super::errors::HlsErrorValue;
// use super::flv2hls::Flv2HlsRemuxer;

////https://www.jianshu.com/p/d6311f03b81f

pub struct FlvDataReceiver {
    app_name: String,
    stream_name: String,

    event_producer: ChannelEventProducer,
    data_consumer: ChannelDataConsumer,
    media_processor: Flv2HlsRemuxer,
    subscriber_id: Uuid,
}

impl FlvDataReceiver {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: ChannelEventProducer,

        duration: i64,
    ) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();
        let subscriber_id = Uuid::new_v4();

        Self {
            app_name: app_name.clone(),
            stream_name: stream_name.clone(),

            data_consumer,
            event_producer,
            media_processor: Flv2HlsRemuxer::new(duration, app_name, stream_name),
            subscriber_id,
        }
    }

    pub async fn run(&mut self) -> Result<(), HlsError> {
        self.subscribe_from_rtmp_channels(self.app_name.clone(), self.stream_name.clone())
            .await?;
        self.receive_flv_data().await?;

        Ok(())
    }

    pub async fn receive_flv_data(&mut self) -> Result<(), HlsError> {
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                let flv_data: FlvData;

                match data {
                    ChannelData::Audio { timestamp, data } => {
                        flv_data = FlvData::Audio { timestamp, data };
                    }
                    ChannelData::Video { timestamp, data } => {
                        flv_data = FlvData::Video { timestamp, data };
                    }
                    _ => continue,
                }

                self.media_processor.process_flv_data(flv_data)?;
            }
        }
    }

    pub fn flush_response_data(&mut self) -> Result<(), HlsError> {
        Ok(())
    }

    pub async fn subscribe_from_rtmp_channels(
        &mut self,
        app_name: String,
        stream_name: String,
    ) -> Result<(), HlsError> {
        let mut retry_count: u8 = 0;

        loop {
            let (sender, receiver) = oneshot::channel();

            let session_info = SessionInfo {
                subscriber_id: self.subscriber_id,
                session_sub_type: SessionSubType::Player,
            };

            let subscribe_event = ChannelEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
                session_info: session_info,
                responder: sender,
            };

            let rv = self.event_producer.send(subscribe_event);
            match rv {
                Err(_) => {
                    let session_error = SessionError {
                        value: SessionErrorValue::SendChannelDataErr,
                    };
                    return Err(HlsError {
                        value: HlsErrorValue::SessionError(session_error),
                    });
                }
                _ => {}
            }

            match receiver.await {
                Ok(consumer) => {
                    self.data_consumer = consumer;
                    break;
                }
                Err(_) => {
                    if retry_count > 10 {
                        let session_error = SessionError {
                            value: SessionErrorValue::SubscribeCountLimitReach,
                        };
                        return Err(HlsError {
                            value: HlsErrorValue::SessionError(session_error),
                        });
                    }
                }
            }

            sleep(Duration::from_millis(800)).await;
            retry_count = retry_count + 1;
        }

        Ok(())
    }
}
