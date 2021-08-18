use {
    super::{
        define::{tag_type, HttpResponseDataProducer},
        errors::{HttpFLvError, HttpFLvErrorValue},
    },
    crate::rtmp::{
        cache::metadata::MetaData,
        channels::define::{ChannelData, ChannelDataConsumer, ChannelEvent, ChannelEventProducer},
        session::{
            common::SessionInfo,
            define::SessionSubType,
            errors::{SessionError, SessionErrorValue},
        },
    },
    bytes::BytesMut,
    std::time::Duration,
    tokio::{
        sync::{mpsc, oneshot},
        time::sleep,
    },
    uuid::Uuid,
    xflv::muxer::{FlvMuxer, HEADER_LENGTH},
};

pub struct HttpFlv {
    app_name: String,
    stream_name: String,

    muxer: FlvMuxer,

    event_producer: ChannelEventProducer,
    data_consumer: ChannelDataConsumer,
    http_response_data_producer: HttpResponseDataProducer,
    subscriber_id: Uuid,
}

impl HttpFlv {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: ChannelEventProducer,
        http_response_data_producer: HttpResponseDataProducer,
    ) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();
        let subscriber_id = Uuid::new_v4();

        Self {
            app_name,
            stream_name,
            muxer: FlvMuxer::new(),
            data_consumer,
            event_producer,
            http_response_data_producer,
            subscriber_id,
        }
    }

    pub async fn run(&mut self) -> Result<(), HttpFLvError> {
        self.subscribe_from_rtmp_channels().await?;
        self.send_media_stream().await?;

        Ok(())
    }

    pub async fn send_media_stream(&mut self) -> Result<(), HttpFLvError> {
        self.muxer.write_flv_header()?;
        self.muxer.write_previous_tag_size(0)?;

        self.flush_response_data()?;
        let mut retry_count = 0;
        //write flv body
        loop {
            if let Some(data) = self.data_consumer.recv().await {
                if let Err(err) = self.write_flv_tag(data) {
                    log::error!("write_flv_tag err: {}", err);
                    retry_count += 1;
                } else {
                    retry_count = 0;
                }
            } else {
                retry_count += 1;
            }
            if retry_count > 10 {
                break;
            }
        }
        self.unsubscribe_from_rtmp_channels().await
    }

    pub fn write_flv_tag(&mut self, channel_data: ChannelData) -> Result<(), HttpFLvError> {
        let common_data: BytesMut;
        let common_timestamp: u32;
        let tag_type: u8;

        match channel_data {
            ChannelData::Audio { timestamp, data } => {
                common_data = data;
                common_timestamp = timestamp;
                tag_type = tag_type::AUDIO;
            }

            ChannelData::Video { timestamp, data } => {
                common_data = data;
                common_timestamp = timestamp;
                tag_type = tag_type::VIDEO;
            }

            ChannelData::MetaData { timestamp, data } => {
                let mut metadata = MetaData::default();
                metadata.save(data);
                let data = metadata.remove_set_data_frame()?;

                common_data = data;
                common_timestamp = timestamp;
                tag_type = tag_type::SCRIPT_DATA_AMF;
            }
        }

        let common_data_len = common_data.len() as u32;

        self.muxer
            .write_flv_tag_header(tag_type, common_data_len, common_timestamp)?;
        self.muxer.write_flv_tag_body(common_data)?;
        self.muxer
            .write_previous_tag_size(common_data_len + HEADER_LENGTH)?;

        self.flush_response_data()?;

        Ok(())
    }

    pub fn flush_response_data(&mut self) -> Result<(), HttpFLvError> {
        let data = self.muxer.writer.extract_current_bytes();
        self.http_response_data_producer.start_send(Ok(data))?;

        Ok(())
    }

    pub async fn unsubscribe_from_rtmp_channels(&mut self) -> Result<(), HttpFLvError> {
        let session_info = SessionInfo {
            subscriber_id: self.subscriber_id,
            session_sub_type: SessionSubType::Player,
        };

        let subscribe_event = ChannelEvent::UnSubscribe {
            app_name: self.app_name.clone(),
            stream_name: self.stream_name.clone(),
            session_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}\n", err);
        }

        Ok(())
    }

    pub async fn subscribe_from_rtmp_channels(&mut self) -> Result<(), HttpFLvError> {
        let mut retry_count: u8 = 0;

        loop {
            let (sender, receiver) = oneshot::channel();

            let session_info = SessionInfo {
                subscriber_id: self.subscriber_id,
                session_sub_type: SessionSubType::Player,
            };

            let subscribe_event = ChannelEvent::Subscribe {
                app_name: self.app_name.clone(),
                stream_name: self.stream_name.clone(),
                session_info: session_info,
                responder: sender,
            };

            let rv = self.event_producer.send(subscribe_event);
            match rv {
                Err(_) => {
                    let session_error = SessionError {
                        value: SessionErrorValue::SendChannelDataErr,
                    };
                    return Err(HttpFLvError {
                        value: HttpFLvErrorValue::SessionError(session_error),
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
                        return Err(HttpFLvError {
                            value: HttpFLvErrorValue::SessionError(session_error),
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
