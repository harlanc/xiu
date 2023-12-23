use tokio::sync::oneshot;
use {
    super::{
        define::{tag_type, HttpResponseDataProducer},
        errors::{HttpFLvError, HttpFLvErrorValue},
    },
    crate::rtmp::{
        cache::metadata::MetaData,
        session::errors::{SessionError, SessionErrorValue},
    },
    bytes::BytesMut,
    std::net::SocketAddr,
    streamhub::define::{
        FrameData, FrameDataReceiver, NotifyInfo, StreamHubEvent, StreamHubEventSender,
        SubDataType, SubscribeType, SubscriberInfo,
    },
    streamhub::{
        stream::StreamIdentifier,
        utils::{RandomDigitCount, Uuid},
    },
    tokio::sync::mpsc,
    xflv::muxer::{FlvMuxer, HEADER_LENGTH},
};

pub struct HttpFlv {
    app_name: String,
    stream_name: String,

    muxer: FlvMuxer,

    event_producer: StreamHubEventSender,
    data_consumer: FrameDataReceiver,
    http_response_data_producer: HttpResponseDataProducer,
    subscriber_id: Uuid,
    request_url: String,
    remote_addr: SocketAddr,
}

impl HttpFlv {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: StreamHubEventSender,
        http_response_data_producer: HttpResponseDataProducer,
        request_url: String,
        remote_addr: SocketAddr,
    ) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();
        let subscriber_id = Uuid::new(RandomDigitCount::Four);

        Self {
            app_name,
            stream_name,
            muxer: FlvMuxer::new(),
            data_consumer,
            event_producer,
            http_response_data_producer,
            subscriber_id,
            request_url,
            remote_addr,
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
                    if let HttpFLvErrorValue::MpscSendError(err_in) = &err.value {
                        if err_in.is_disconnected() {
                            log::info!("write_flv_tag: {}", err_in);
                            break;
                        }
                    }
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

    pub fn write_flv_tag(&mut self, channel_data: FrameData) -> Result<(), HttpFLvError> {
        let (common_data, common_timestamp, tag_type) = match channel_data {
            FrameData::Audio { timestamp, data } => (data, timestamp, tag_type::AUDIO),
            FrameData::Video { timestamp, data } => (data, timestamp, tag_type::VIDEO),
            FrameData::MetaData { timestamp, data } => {
                let mut metadata = MetaData::new();
                metadata.save(&data);
                let data = metadata.remove_set_data_frame()?;

                (data, timestamp, tag_type::SCRIPT_DATA_AMF)
            }
            _ => {
                log::error!("should not be here!!!");
                (BytesMut::new(), 0, 0)
            }
        };

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
        let sub_info = SubscriberInfo {
            id: self.subscriber_id,
            sub_type: SubscribeType::PlayerHttpFlv,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtmp {
            app_name: self.app_name.clone(),
            stream_name: self.stream_name.clone(),
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}", err);
        }

        Ok(())
    }

    pub async fn subscribe_from_rtmp_channels(&mut self) -> Result<(), HttpFLvError> {
        let sub_info = SubscriberInfo {
            id: self.subscriber_id,
            sub_type: SubscribeType::PlayerHttpFlv,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtmp {
            app_name: self.app_name.clone(),
            stream_name: self.stream_name.clone(),
        };

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: sub_info,
            result_sender: event_result_sender,
        };

        let rv = self.event_producer.send(subscribe_event);
        if rv.is_err() {
            let session_error = SessionError {
                value: SessionErrorValue::SendFrameDataErr,
            };
            return Err(HttpFLvError {
                value: HttpFLvErrorValue::SessionError(session_error),
            });
        }

        let receiver = event_result_receiver.await??.frame_receiver.unwrap();
        self.data_consumer = receiver;

        Ok(())
    }
}
