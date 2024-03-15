use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use h264_decoder::sps::SpsParser;
use streamhub::define::VideoCodecType;
use tokio::sync::oneshot;
use xflv::define::h264_nal_type::{H264_NAL_IDR, H264_NAL_PPS, H264_NAL_SPS};

use crate::session::define::SessionType;

use super::{
    errors::{RtmpRemuxerError, RtmpRemuxerErrorValue},
    rtmp_cooker::RtmpCooker,
};

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
pub struct Whip2RtmpRemuxerSession {
    event_producer: StreamHubEventSender,
    //RTMP
    app_name: String,
    stream_name: String,

    //WHIP
    data_receiver: FrameDataReceiver,

    subscribe_id: Uuid,
    video_clock_rate: u32,
    audio_clock_rate: u32,
    //because
    base_video_timestamp: u32,
    base_audio_timestamp: u32,

    rtmp_handler: Common,
    rtmp_cooker: RtmpCooker,

    sps: Option<BytesMut>,
    pps: Option<BytesMut>,
    video_seq_header_generated: bool,
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

pub fn print(data: BytesMut) {
    println!("==========={}", data.len());
    let mut idx = 0;
    for i in data {
        print!("{i:02X} ");
        idx += 1;
        if idx % 16 == 0 {
            println!()
        }
    }

    println!("===========")
}

impl Whip2RtmpRemuxerSession {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: StreamHubEventSender,
    ) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();

        Self {
            app_name,
            stream_name,
            data_receiver: data_consumer,
            event_producer: event_producer.clone(),

            subscribe_id: Uuid::new(RandomDigitCount::Four),
            video_clock_rate: 1000,
            audio_clock_rate: 1000,
            base_audio_timestamp: 0,
            base_video_timestamp: 0,
            rtmp_handler: Common::new(None, event_producer, SessionType::Server, None),
            rtmp_cooker: RtmpCooker::default(),
            sps: None,
            pps: None,
            video_seq_header_generated: false,
        }
    }

    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        self.publish_rtmp().await?;
        self.subscribe_whip().await?;
        self.receive_whip_data().await?;

        Ok(())
    }

    pub async fn publish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .publish_to_channels(self.app_name.clone(), self.stream_name.clone(), 1)
            .await?;
        Ok(())
    }

    pub async fn unpublish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .unpublish_to_channels(self.app_name.clone(), self.stream_name.clone())
            .await?;
        Ok(())
    }

    pub async fn subscribe_whip(&mut self) -> Result<(), RtmpRemuxerError> {
        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtmp,
            sub_data_type: streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::WebRTC {
                app_name: self.app_name.clone(),
                stream_name: self.stream_name.clone(),
            },
            info: sub_info,
            result_sender: event_result_sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(RtmpRemuxerError {
                value: RtmpRemuxerErrorValue::StreamHubEventSendErr,
            });
        }

        let receiver = event_result_receiver.await??.0;
        self.data_receiver = receiver.frame_receiver.unwrap();
        Ok(())
    }

    pub async fn unsubscribe_whip(&mut self) -> Result<(), RtmpRemuxerError> {
        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtmp,
            sub_data_type: streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier: StreamIdentifier::WebRTC {
                app_name: self.app_name.clone(),
                stream_name: self.stream_name.clone(),
            },
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}", err);
        }

        Ok(())
    }

    pub async fn receive_whip_data(&mut self) -> Result<(), RtmpRemuxerError> {
        let mut retry_count = 0;
        log::info!("begin receive whip data...");
        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    FrameData::Audio { timestamp, data } => {
                        self.on_whip_audio(&data, timestamp).await?
                    }
                    FrameData::Video {
                        timestamp,
                        mut data,
                    } => {
                        self.on_whip_video(&mut data, timestamp).await?;
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

        self.unsubscribe_whip().await?;
        self.unpublish_rtmp().await
    }

    async fn on_whip_audio(
        &mut self,
        audio_data: &BytesMut,
        timestamp: u32,
    ) -> Result<(), RtmpRemuxerError> {
        if self.base_audio_timestamp == 0 {
            self.base_audio_timestamp = timestamp;
        }

        let mut audio_frame = self.rtmp_cooker.gen_audio_frame_data(audio_data)?;

        let timestamp_adjust =
            (timestamp - self.base_audio_timestamp) / (self.audio_clock_rate / 1000);

        self.rtmp_handler
            .on_audio_data(&mut audio_frame, &timestamp_adjust)
            .await?;

        Ok(())
    }

    async fn on_whip_video(
        &mut self,
        nalus: &mut BytesMut,
        timestamp: u32,
    ) -> Result<(), RtmpRemuxerError> {
        if self.base_video_timestamp == 0 {
            self.base_video_timestamp = timestamp;
        }
        let mut nalu_vec = Vec::new();
        while !nalus.is_empty() {
            if let Some(first_pos) = find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                        let mut second_pos = first_pos + 3 + distance_to_first_pos;
                        while second_pos > 0 && nalus[second_pos - 1] == 0 {
                            second_pos -= 1;
                        }
                        nalus.split_to(second_pos)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                let nalu = nalu_with_start_code.split_off(first_pos + 3);
                nalu_vec.push(nalu);
            } else {
                break;
            }
        }

        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut level: u8 = 0;
        let mut profile: u8 = 0;

        let mut contains_idr = false;

        for nalu in &nalu_vec {
            let mut nalu_reader = BytesReader::new(nalu.clone());

            let nalu_type = nalu_reader.read_u8()?;
            match nalu_type & 0x1F {
                H264_NAL_SPS => {
                    let mut sps_parser = SpsParser::new(nalu_reader);
                    (width, height) = if let Ok((width, height)) = sps_parser.parse() {
                        (width, height)
                    } else {
                        (0, 0)
                    };

                    level = sps_parser.sps.level_idc;
                    profile = sps_parser.sps.profile_idc;

                    self.sps = Some(nalu.clone());
                }
                H264_NAL_PPS => self.pps = Some(nalu.clone()),
                H264_NAL_IDR => {
                    contains_idr = true;
                }
                _ => {}
            }
        }

        nalu_vec.retain(|nalu| {
            let nalu_type = nalu[0] & 0x1F;
            nalu_type != H264_NAL_SPS && nalu_type != H264_NAL_PPS
        });

        if !self.video_seq_header_generated {
            if self.sps.is_some() && self.pps.is_some() {
                let mut meta_data = self.rtmp_cooker.gen_meta_data(width, height)?;
                self.rtmp_handler.on_meta_data(&mut meta_data, &0).await?;

                let mut seq_header = self.rtmp_cooker.gen_video_seq_header(
                    self.sps.clone().unwrap(),
                    self.pps.clone().unwrap(),
                    profile,
                    level,
                )?;
                self.rtmp_handler.on_video_data(&mut seq_header, &0).await?;
                self.video_seq_header_generated = true;
            }
        } else if !nalu_vec.is_empty() {
            let mut frame_data = self
                .rtmp_cooker
                .gen_video_frame_data(nalu_vec, contains_idr)?;

            let timestamp_adjust =
                (timestamp - self.base_video_timestamp) / (self.video_clock_rate / 1000);

            self.rtmp_handler
                .on_video_data(&mut frame_data, &timestamp_adjust)
                .await?;
        }

        Ok(())
    }
}
