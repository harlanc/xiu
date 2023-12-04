use super::{
    errors::{RtmpRemuxerError, RtmpRemuxerErrorValue},
    rtmp_cooker::RtmpCooker,
};
use crate::session::define::SessionType;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use h264_decoder::sps::SpsParser;
use streamhub::define::{DataSender, VideoCodecType};
use xflv::define::h264_nal_type::{H264_NAL_IDR, H264_NAL_PPS, H264_NAL_SPS};

use {
    crate::session::common::Common,
    std::time::Duration,
    streamhub::{
        define::{
            FrameData, FrameDataReceiver, NotifyInfo, StreamHubEvent, StreamHubEventSender,
            SubscribeType, SubscriberInfo,
        },
        stream::StreamIdentifier,
        utils::{RandomDigitCount, Uuid},
    },
    tokio::{sync::mpsc, time::sleep},
};
pub struct GB281812RtmpRemuxerSession {
    event_producer: StreamHubEventSender,
    //RTMP
    app_name: String,
    stream_name: String,

    publishe_id: Uuid,
    //GB28181
    data_receiver: FrameDataReceiver,
    subscribe_id: Uuid,
    video_clock_rate: u32,
    audio_clock_rate: u32,
    base_video_timestamp: u32,
    base_audio_timestamp: u32,
    rtmp_handler: Common,
    rtmp_cooker: RtmpCooker,
    sps: Option<BytesMut>,
    pps: Option<BytesMut>,
    is_seq_header_generated: bool,
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

impl GB281812RtmpRemuxerSession {
    pub fn new(stream_name: String, event_producer: StreamHubEventSender) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();

        Self {
            app_name: String::from("gb28181"),
            stream_name,
            data_receiver: data_consumer,
            event_producer: event_producer.clone(),
            subscribe_id: Uuid::new(RandomDigitCount::Four),
            publishe_id: Uuid::new(RandomDigitCount::Four),
            video_clock_rate: 90 * 1000,
            audio_clock_rate: 90 * 1000,
            base_audio_timestamp: 0,
            base_video_timestamp: 0,
            rtmp_handler: Common::new(None, event_producer, SessionType::Server, None),
            rtmp_cooker: RtmpCooker::default(),
            sps: None,
            pps: None,
            is_seq_header_generated: false,
        }
    }

    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        self.publish_rtmp().await?;
        self.subscribe_gb28181().await?;
        self.receive_gb28181_data().await?;

        Ok(())
    }

    pub async fn publish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .publish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.publishe_id,
                0,
            )
            .await?;
        Ok(())
    }

    pub async fn unpublish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .unpublish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.publishe_id,
            )
            .await?;
        Ok(())
    }

    pub async fn subscribe_gb28181(&mut self) -> Result<(), RtmpRemuxerError> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtmp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::GB28181 {
                stream_name: self.stream_name.clone(),
            },
            info: sub_info,
            sender: DataSender::Frame { sender },
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(RtmpRemuxerError {
                value: RtmpRemuxerErrorValue::StreamHubEventSendErr,
            });
        }

        self.data_receiver = receiver;
        Ok(())
    }

    pub async fn unsubscribe_gb28181(&mut self) -> Result<(), RtmpRemuxerError> {
        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtmp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier: StreamIdentifier::GB28181 {
                stream_name: self.stream_name.clone(),
            },
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}\n", err);
        }

        Ok(())
    }

    pub async fn receive_gb28181_data(&mut self) -> Result<(), RtmpRemuxerError> {
        let mut retry_count = 0;

        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    FrameData::Audio {
                        timestamp,
                        mut data,
                    } => self.on_gb28181_audio(&mut data, timestamp).await?,
                    FrameData::Video { timestamp, data } => {
                        log::info!("video timestamp: {}", timestamp);
                        self.on_gb28181_video(data, timestamp).await?;
                    }
                    FrameData::MediaInfo { media_info } => {
                        self.video_clock_rate = media_info.video_clock_rate;
                        self.audio_clock_rate = media_info.audio_clock_rate;
                        log::info!(
                            "audio clock rate: {} video clock rate: {}",
                            self.audio_clock_rate,
                            self.video_clock_rate
                        );

                        if media_info.vcodec == VideoCodecType::H265 {
                            log::warn!(
                                "h265 rtsp to rtmp is not supported now!!! will come soon!!"
                            );
                            break;
                        }
                    }
                    _ => continue,
                };
                retry_count = 0;
            } else {
                sleep(Duration::from_millis(100)).await;
                retry_count += 1;
            }

            if retry_count > 10 {
                break;
            }
        }

        self.unsubscribe_gb28181().await?;
        self.unpublish_rtmp().await
    }

    async fn on_gb28181_audio(
        &mut self,
        audio_data: &BytesMut,
        timestamp: u32,
    ) -> Result<(), RtmpRemuxerError> {
        if self.base_audio_timestamp == 0 {
            self.base_audio_timestamp = timestamp;
        }
        let audio_frame = self.rtmp_cooker.gen_audio_frame_data(audio_data)?;

        let timestamp_adjust =
            (timestamp - self.base_audio_timestamp) / (self.audio_clock_rate / 1000);
        self.rtmp_handler
            .on_audio_data(&audio_frame, &timestamp_adjust)
            .await?;

        Ok(())
    }

    async fn on_gb28181_video(
        &mut self,
        nalu: BytesMut,
        timestamp: u32,
    ) -> Result<(), RtmpRemuxerError> {
        if self.base_video_timestamp == 0 {
            self.base_video_timestamp = timestamp;
        }

        let mut contains_idr = false;
        let mut nalu_reader = BytesReader::new(nalu.clone());

        let nalu_type = nalu_reader.read_u8()?;
        let mut is_av = true;
        match nalu_type & 0x1F {
            H264_NAL_SPS => {
                self.sps = Some(nalu.clone());
                is_av = false;
            }
            H264_NAL_PPS => {
                log::info!("receive PPS...");
                self.pps = Some(nalu.clone());
                is_av = false;
            }
            H264_NAL_IDR => {
                contains_idr = true;
            }
            _ => {}
        }

        // the first sps + pps + idr frame compose the SEQ header
        if self.sps.is_some() && self.pps.is_some() && !self.is_seq_header_generated {
            let width: u32;
            let height: u32;

            let mut sps_parser = SpsParser::new(BytesReader::new(self.sps.clone().unwrap()));
            (width, height) = if let Ok((width, height)) = sps_parser.parse() {
                (width, height)
            } else {
                (0, 0)
            };

            log::info!("receive SPS...width:{}x{}", width, height);
            let level = sps_parser.sps.level_idc;
            let profile = sps_parser.sps.profile_idc;
            let mut meta_data = self.rtmp_cooker.gen_meta_data(width, height)?;
            self.rtmp_handler.on_meta_data(&mut meta_data, &0).await?;

            let mut seq_header = self.rtmp_cooker.gen_video_seq_header(
                self.sps.clone().unwrap(),
                self.pps.clone().unwrap(),
                profile,
                level,
            )?;
            self.sps = None;
            self.pps = None;
            self.rtmp_handler.on_video_data(&mut seq_header, &0).await?;
            self.is_seq_header_generated = true;
        } else if is_av {
            let mut frame_data = self
                .rtmp_cooker
                .gen_video_frame_data(vec![nalu], contains_idr)?;

            let timestamp_adjust =
                (timestamp - self.base_video_timestamp) / (self.video_clock_rate / 1000);
            self.rtmp_handler
                .on_video_data(&mut frame_data, &timestamp_adjust)
                .await?;
        }

        Ok(())
    }
}
