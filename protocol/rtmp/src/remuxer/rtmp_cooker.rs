use bytes::BytesMut;
use bytesio::bytes_writer::BytesWriter;
use indexmap::IndexMap;

use xflv::{
    flv_tag_header::{AudioTagHeader, VideoTagHeader},
    mpeg4_avc::{Mpeg4Avc, Mpeg4AvcProcessor, Pps, Sps},
    Marshal,
};

use super::errors::RtmpRemuxerError;
use crate::amf0::{amf0_writer::Amf0Writer, Amf0ValueType};

#[derive(Default)]
pub struct RtmpCooker {}

impl RtmpCooker {
    pub fn gen_meta_data(&self, width: u32, height: u32) -> Result<BytesMut, RtmpRemuxerError> {
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
    pub fn gen_video_seq_header(
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

    pub fn gen_video_frame_data(
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
    //generate audio rtmp frame (including seq header and common frame)
    pub fn gen_audio_frame_data(
        &self,
        audio_data: &BytesMut,
    ) -> Result<BytesMut, RtmpRemuxerError> {
        let mut aac_packet_type: u8 = 0;

        if audio_data.len() > 5 {
            aac_packet_type = 1;
        }

        let audio_tag_header = AudioTagHeader {
            sound_format: 10,
            sound_rate: 3,
            sound_size: 1,
            sound_type: 1,
            aac_packet_type,
        };

        let tag_header_data = audio_tag_header.marshal()?;

        let mut writer = BytesWriter::new();
        writer.write(&tag_header_data)?;
        writer.write(audio_data)?;

        Ok(writer.extract_current_bytes())
    }
}
