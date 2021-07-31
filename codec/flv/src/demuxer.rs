use super::define::aac_packet_type;
use super::define::avc_packet_type;
use super::define::codec_id;
use super::define::sound_format;
use super::define::FlvDemuxerData;
use super::demuxer_tag::AudioTagHeaderDemuxer;
use super::demuxer_tag::VideoTagHeaderDemuxer;
use super::errors::FlvDemuxerError;
use super::mpeg4_aac::Mpeg4AacProcessor;
use super::mpeg4_avc::Mpeg4AvcProcessor;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;

pub struct FlvDemuxerAudioData {
    pub has_data: bool,
    pub sound_format: u8,
    pub dts: u64,
    pub pts: u64,
    pub data: BytesMut,
}

impl FlvDemuxerAudioData {
    pub fn default() -> Self {
        Self {
            has_data: false,
            sound_format: 0,
            dts: 0,
            pts: 0,
            data: BytesMut::new(),
        }
    }
}

pub struct FlvDemuxerVideoData {
    pub has_data: bool,
    pub codec_id: u8,
    pub dts: u64,
    pub pts: u64,
    pub frame_type: u8,
    pub data: BytesMut,
}

impl FlvDemuxerVideoData {
    pub fn default() -> Self {
        Self {
            has_data: false,
            codec_id: 0,
            dts: 0,
            pts: 0,
            frame_type: 0,
            data: BytesMut::new(),
        }
    }
}

pub struct FlvVideoDemuxer {
    avc_processor: Mpeg4AvcProcessor,
}

impl FlvVideoDemuxer {
    pub fn new() -> Self {
        Self {
            avc_processor: Mpeg4AvcProcessor::new(),
        }
    }
    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<FlvDemuxerVideoData, FlvDemuxerError> {
        let mut video_tag_demuxer = VideoTagHeaderDemuxer::new(data);
        let header = video_tag_demuxer.parse_tag_header()?;
        let remaining_bytes = video_tag_demuxer.get_remaining_bytes();
        let cts = header.composition_time;

        self.avc_processor.extend_data(remaining_bytes);

        match header.codec_id {
            codec_id::FLV_VIDEO_H264 => match header.avc_packet_type {
                avc_packet_type::AVC_SEQHDR => {
                    self.avc_processor.decoder_configuration_record_load()?;
                    return Ok(FlvDemuxerVideoData::default());
                }
                avc_packet_type::AVC_NALU => {
                    self.avc_processor.h264_mp4toannexb()?;

                    let video_data = FlvDemuxerVideoData {
                        has_data: true,
                        codec_id: codec_id::FLV_VIDEO_H264,
                        pts: timestamp as u64 + cts as u64,
                        dts: timestamp as u64,
                        frame_type: header.frame_type,
                        data: self.avc_processor.bytes_writer.extract_current_bytes(),
                    };
                    return Ok(video_data);
                }
                _ => {}
            },

            _ => {}
        }

        Ok(FlvDemuxerVideoData::default())
    }
}

pub struct FlvAudioDemuxer {
    aac_processor: Mpeg4AacProcessor,
}

impl FlvAudioDemuxer {
    pub fn new() -> Self {
        Self {
            aac_processor: Mpeg4AacProcessor::new(),
        }
    }

    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<FlvDemuxerAudioData, FlvDemuxerError> {
        let mut audio_tag_demuxer = AudioTagHeaderDemuxer::new(data);
        let header = audio_tag_demuxer.parse_tag_header()?;
        let remaining_bytes = audio_tag_demuxer.get_remaining_bytes();

        self.aac_processor.extend_data(remaining_bytes);

        match header.sound_format {
            sound_format::AAC => match header.aac_packet_type {
                aac_packet_type::AAC_SEQHDR => {
                    self.aac_processor.audio_specific_config_load()?;
                    return Ok(FlvDemuxerAudioData::default());
                }
                aac_packet_type::AAC_RAW => {
                    self.aac_processor.adts_save()?;

                    let audio_data = FlvDemuxerAudioData {
                        has_data: true,
                        sound_format: header.sound_format,
                        pts: timestamp as u64,
                        dts: timestamp as u64,
                        data: self.aac_processor.bytes_writer.extract_current_bytes(),
                    };
                    return Ok(audio_data);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(FlvDemuxerAudioData::default())
    }
}
