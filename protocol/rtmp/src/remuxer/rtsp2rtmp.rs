use bytes::BytesMut;
use bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter};
use h264_decoder::sps::SpsParser;
use indexmap::IndexMap;
use streamhub::define::VideoCodecType;
use tokio::sync::oneshot;
use xflv::{
    define::h264_nal_type::{H264_NAL_IDR, H264_NAL_PPS, H264_NAL_SPS},
    flv_tag_header::{AudioTagHeader, VideoTagHeader},
    mpeg4_avc::{Mpeg4Avc, Mpeg4AvcProcessor, Pps, Sps},
    Marshal,
};

use crate::{
    amf0::{amf0_writer::Amf0Writer, Amf0ValueType},
    session::define::SessionType,
};

use super::errors::{RtmpRemuxerError, RtmpRemuxerErrorValue};

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
pub struct Rtsp2RtmpRemuxerSession {
    event_producer: StreamHubEventSender,
    //RTMP
    app_name: String,
    stream_name: String,
    rtmp_handler: Common,
    publishe_id: Uuid,
    //RTSP
    data_receiver: FrameDataReceiver,
    stream_path: String,
    subscribe_id: Uuid,
    video_clock_rate: u32,
    audio_clock_rate: u32,
    base_video_timestamp: u32,
    base_audio_timestamp: u32,
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

impl Rtsp2RtmpRemuxerSession {
    pub fn new(stream_path: String, event_producer: StreamHubEventSender) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();

        let eles: Vec<&str> = stream_path.splitn(2, '/').collect();
        let (app_name, stream_name) = if eles.len() < 2 {
            log::warn!(
                "publish_rtmp: the rtsp path only contains stream name: {}",
                stream_path
            );
            (String::from("rtsp"), String::from(eles[0]))
        } else {
            (String::from(eles[0]), String::from(eles[1]))
        };

        Self {
            stream_path,
            app_name,
            stream_name,
            data_receiver: data_consumer,
            event_producer: event_producer.clone(),
            rtmp_handler: Common::new(None, event_producer, SessionType::Server, None),
            subscribe_id: Uuid::new(RandomDigitCount::Four),
            publishe_id: Uuid::new(RandomDigitCount::Four),
            video_clock_rate: 1000,
            audio_clock_rate: 1000,
            base_audio_timestamp: 0,
            base_video_timestamp: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        self.publish_rtmp().await?;
        self.subscribe_rtsp().await?;
        self.receive_rtsp_data().await?;

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

    pub async fn subscribe_rtsp(&mut self) -> Result<(), RtmpRemuxerError> {
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
            identifier: StreamIdentifier::Rtsp {
                stream_path: self.stream_path.clone(),
            },
            info: sub_info,
            result_sender: event_result_sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(RtmpRemuxerError {
                value: RtmpRemuxerErrorValue::StreamHubEventSendErr,
            });
        }

        let receiver = event_result_receiver.await??;
        self.data_receiver = receiver.frame_receiver.unwrap();
        Ok(())
    }

    pub async fn unsubscribe_rtsp(&mut self) -> Result<(), RtmpRemuxerError> {
        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtsp,
            sub_data_type: streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: self.stream_path.clone(),
            },
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}", err);
        }

        Ok(())
    }

    pub async fn receive_rtsp_data(&mut self) -> Result<(), RtmpRemuxerError> {
        let mut retry_count = 0;

        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    FrameData::Audio {
                        timestamp,
                        mut data,
                    } => self.on_rtsp_audio(&mut data, timestamp).await?,
                    FrameData::Video {
                        timestamp,
                        mut data,
                    } => {
                        self.on_rtsp_video(&mut data, timestamp).await?;
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

        self.unsubscribe_rtsp().await?;
        self.unpublish_rtmp().await
    }

    async fn on_rtsp_audio(
        &mut self,
        audio_data: &mut BytesMut,
        timestamp: u32,
    ) -> Result<(), RtmpRemuxerError> {
        if self.base_audio_timestamp == 0 {
            self.base_audio_timestamp = timestamp;
        }
        let mut audio_tag_header = AudioTagHeader {
            sound_format: 10,
            sound_rate: 3,
            sound_size: 1,
            sound_type: 1,
            aac_packet_type: 0,
        };

        if audio_data.len() > 5 {
            audio_tag_header.aac_packet_type = 1;
        }

        let tag_header_data = audio_tag_header.marshal()?;

        let mut writer = BytesWriter::new();
        writer.write(&tag_header_data)?;
        writer.write(audio_data)?;

        let timestamp_adjust =
            (timestamp - self.base_audio_timestamp) / (self.audio_clock_rate / 1000);
        self.rtmp_handler
            .on_audio_data(&mut writer.extract_current_bytes(), &timestamp_adjust)
            .await?;

        Ok(())
    }

    async fn on_rtsp_video(
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
        let mut sps = None;
        let mut pps = None;
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

                    log::info!("width:{}x{}", width, height);
                    level = sps_parser.sps.level_idc;
                    profile = sps_parser.sps.profile_idc;

                    sps = Some(nalu.clone());
                }
                H264_NAL_PPS => pps = Some(nalu.clone()),
                H264_NAL_IDR => {
                    contains_idr = true;
                }
                _ => {}
            }
        }

        if sps.is_some() && pps.is_some() {
            let mut meta_data = self.gen_rtmp_meta_data(width, height)?;
            self.rtmp_handler.on_meta_data(&mut meta_data, &0).await?;

            let mut seq_header =
                self.gen_rtmp_video_seq_header(sps.unwrap(), pps.unwrap(), profile, level)?;
            self.rtmp_handler.on_video_data(&mut seq_header, &0).await?;
        } else {
            let mut frame_data = self.gen_rtmp_video_frame_data(nalu_vec, contains_idr)?;

            let timestamp_adjust =
                (timestamp - self.base_video_timestamp) / (self.video_clock_rate / 1000);
            self.rtmp_handler
                .on_video_data(&mut frame_data, &timestamp_adjust)
                .await?;
        }

        Ok(())
    }

    fn gen_rtmp_meta_data(&self, width: u32, height: u32) -> Result<BytesMut, RtmpRemuxerError> {
        let mut amf_writer = Amf0Writer::new();
        amf_writer.write_string(&String::from("@setDataFrame"))?;
        amf_writer.write_string(&String::from("onMetaData"))?;

        let mut properties = IndexMap::new();
        properties.insert(String::from("width"), Amf0ValueType::Number(width as f64));
        properties.insert(String::from("height"), Amf0ValueType::Number(height as f64));
        properties.insert(String::from("videocodecid"), Amf0ValueType::Number(7.));
        properties.insert(String::from("audiocodecid"), Amf0ValueType::Number(10.));
        amf_writer.write_eacm_array(&properties)?;

        Ok(amf_writer.extract_current_bytes())
    }
    fn gen_rtmp_video_seq_header(
        &self,
        sps: BytesMut,
        pps: BytesMut,
        profile: u8,
        level: u8,
    ) -> Result<BytesMut, RtmpRemuxerError> {
        let video_tag_header = VideoTagHeader {
            frame_type: 1,
            codec_id: 7,
            avc_packet_type: 0,
            composition_time: 0,
        };
        let tag_header_data = video_tag_header.marshal()?;

        let mut processor = Mpeg4AvcProcessor {
            mpeg4_avc: Mpeg4Avc {
                profile,
                compatibility: 0,
                level,
                nalu_length: 4,
                nb_pps: 1,
                sps: vec![Sps { data: sps }],
                nb_sps: 1,
                pps: vec![Pps { data: pps }],
                ..Default::default()
            },
        };
        let mpegavc_data = processor.decoder_configuration_record_save()?;

        let mut writer = BytesWriter::new();
        writer.write(&tag_header_data)?;
        writer.write(&mpegavc_data)?;

        Ok(writer.extract_current_bytes())
    }

    fn gen_rtmp_video_frame_data(
        &self,
        nalus: Vec<BytesMut>,
        contains_idr: bool,
    ) -> Result<BytesMut, RtmpRemuxerError> {
        let frame_type = if contains_idr { 1 } else { 2 };
        let video_tag_header = VideoTagHeader {
            frame_type,
            codec_id: 7,
            avc_packet_type: 1,
            composition_time: 0,
        };
        let tag_header_data = video_tag_header.marshal()?;

        let mut processor = Mpeg4AvcProcessor {
            mpeg4_avc: Mpeg4Avc {
                nalu_length: 4,
                ..Default::default()
            },
        };
        let mpegavc_data = processor.nalus_to_mpeg4avc(nalus)?;

        let mut writer = BytesWriter::new();
        writer.write(&tag_header_data)?;
        writer.write(&mpegavc_data)?;

        Ok(writer.extract_current_bytes())
    }
}
