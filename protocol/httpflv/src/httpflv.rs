use streamhub::define::{StatisticData, StatisticDataSender};
use tokio::sync::oneshot;
use {
    super::{
        define::{tag_type, HttpResponseDataProducer},
        errors::{HttpFLvError, HttpFLvErrorValue},
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
    xflv::amf0::amf0_writer::Amf0Writer,
    xflv::muxer::{FlvMuxer, HEADER_LENGTH},
};

pub struct HttpFlv {
    app_name: String,
    stream_name: String,

    muxer: FlvMuxer,

    has_audio: bool,
    has_video: bool,
    has_send_header: bool,

    event_producer: StreamHubEventSender,
    data_receiver: FrameDataReceiver,
    /* now used for subscriber session */
    statistic_data_sender: Option<StatisticDataSender>,
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
        let (_, data_receiver) = mpsc::unbounded_channel();
        let subscriber_id = Uuid::new(RandomDigitCount::Four);

        Self {
            app_name,
            stream_name,
            muxer: FlvMuxer::new(),
            has_audio: false,
            has_video: false,
            has_send_header: false,
            data_receiver,
            statistic_data_sender: None,
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
        let mut retry_count = 0;

        let mut max_av_frame_num_to_guess_av = 0;
        // the first av frames are sequence configs, must be cached;
        let mut cached_frames = Vec::new();
        //write flv body
        loop {
            if let Some(data) = self.data_receiver.recv().await {
                if !self.has_send_header {
                    max_av_frame_num_to_guess_av += 1;

                    match data {
                        FrameData::Audio {
                            timestamp: _,
                            data: _,
                        } => {
                            self.has_audio = true;
                            cached_frames.push(data);
                        }
                        FrameData::Video {
                            timestamp: _,
                            data: _,
                        } => {
                            self.has_video = true;
                            cached_frames.push(data);
                        }
                        FrameData::MetaData {
                            timestamp: _,
                            data: _,
                        } => cached_frames.push(data),
                        _ => {}
                    }

                    if (self.has_audio && self.has_video) || max_av_frame_num_to_guess_av > 10 {
                        self.has_send_header = true;
                        self.muxer
                            .write_flv_header(self.has_audio, self.has_video)?;
                        self.muxer.write_previous_tag_size(0)?;

                        self.flush_response_data()?;

                        for frame in &cached_frames {
                            self.write_flv_tag(frame.clone())?;
                        }
                        cached_frames.clear();
                    }

                    continue;
                }

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

    //used for the http-flv protocol

    pub fn write_flv_tag(&mut self, channel_data: FrameData) -> Result<(), HttpFLvError> {
        let (common_data, common_timestamp, tag_type) = match channel_data {
            FrameData::Audio { timestamp, data } => {
                if let Some(sender) = &self.statistic_data_sender {
                    let statistic_audio_data = StatisticData::Audio {
                        uuid: Some(self.subscriber_id),
                        aac_packet_type: 1,
                        data_size: data.len(),
                        duration: 0,
                    };
                    if let Err(err) = sender.send(statistic_audio_data) {
                        log::error!("send statistic data err: {}", err);
                    }
                }

                (data, timestamp, tag_type::AUDIO)
            }
            FrameData::Video { timestamp, data } => {
                if let Some(sender) = &self.statistic_data_sender {
                    let statistic_video_data = StatisticData::Video {
                        uuid: Some(self.subscriber_id),
                        frame_count: 1,
                        is_key_frame: None,
                        data_size: data.len(),
                        duration: 0,
                    };
                    if let Err(err) = sender.send(statistic_video_data) {
                        log::error!("send statistic data err: {}", err);
                    }
                }

                (data, timestamp, tag_type::VIDEO)
            }
            FrameData::MetaData { timestamp, data } => {
                //remove @setDataFrame from RTMP's metadata
                let mut amf_writer: Amf0Writer = Amf0Writer::new();
                amf_writer.write_string(&String::from("@setDataFrame"))?;
                let (_, right) = data.split_at(amf_writer.len());

                (BytesMut::from(right), timestamp, tag_type::SCRIPT_DATA_AMF)
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
            return Err(HttpFLvError {
                value: HttpFLvErrorValue::SendFrameDataErr,
            });
        }

        let result_receiver = event_result_receiver.await??;
        let receiver = result_receiver.0.frame_receiver.unwrap();
        self.data_receiver = receiver;
        self.statistic_data_sender = result_receiver.1;

        if let Some(sender) = &self.statistic_data_sender {
            let statistic_subscriber = StatisticData::Subscriber {
                id: self.subscriber_id,
                remote_addr: self.remote_addr.to_string(),
                start_time: chrono::Local::now(),
                sub_type: SubscribeType::PlayerHttpFlv,
            };
            if let Err(err) = sender.send(statistic_subscriber) {
                log::error!("send statistic_subscriber err: {}", err);
            }
        }

        Ok(())
    }
}
