use bytes::BytesMut;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::sync::Mutex;

use flv::{define, tag_parser};
use netio::netio::NetworkIO;

use crate::amf0::Amf0ValueType;
use crate::chunk::define as chunk_define;
use crate::chunk::packetizer::ChunkPacketizer;
use crate::chunk::ChunkInfo;
use crate::messages::define as messages_define;

use super::errors::CacheError;
use super::metadata;

pub struct Cache {
    metadata: metadata::MetaData,
    video_seq: BytesMut,
    video_timestamp: u32,
    audio_seq: BytesMut,
    audio_timestamp: u32,
    packetizer: ChunkPacketizer,
}

impl Cache {
    fn new(io: Arc<Mutex<NetworkIO>>) -> Self {
        Self {
            metadata: metadata::MetaData::default(),
            video_seq: BytesMut::new(),
            video_timestamp: 0,
            audio_seq: BytesMut::new(),
            audio_timestamp: 0,
            packetizer: ChunkPacketizer::new(io),
        }
    }

    async fn write_amf_data_chunk(&mut self) -> Result<(), CacheError> {
        let data = self.metadata.get_chunk_body();
        let mut chunk_info = ChunkInfo::new(
            chunk_define::csid_type::DATA_AMF0_AMF3,
            chunk_define::chunk_type::TYPE_0,
            0,
            data.len() as u32,
            messages_define::msg_type_id::DATA_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }

    async fn write_video_seq_chunk(&mut self) -> Result<(), CacheError> {
        let mut chunk_info = ChunkInfo::new(
            chunk_define::csid_type::VIDEO,
            chunk_define::chunk_type::TYPE_0,
            self.video_timestamp,
            self.video_seq.len() as u32,
            messages_define::msg_type_id::VIDEO,
            0,
            self.video_seq.clone(),
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }

    async fn write_audio_seq_chunk(&mut self) -> Result<(), CacheError> {
        let mut chunk_info = ChunkInfo::new(
            chunk_define::csid_type::AUDIO,
            chunk_define::chunk_type::TYPE_0,
            self.audio_timestamp,
            self.audio_seq.len() as u32,
            messages_define::msg_type_id::AUDIO,
            0,
            self.audio_seq.clone(),
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }

    pub async fn send(&mut self) -> Result<(), CacheError> {
        self.write_amf_data_chunk().await?;
        self.write_video_seq_chunk().await?;
        self.write_audio_seq_chunk().await?;
        Ok(())
    }

    pub fn save_metadata(&mut self, chunk_body: &mut BytesMut, values: &mut Vec<Amf0ValueType>) {
        self.metadata.save(chunk_body, values);
    }

    pub fn save_audio_seq(
        &mut self,
        chunk_body: &mut BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = tag_parser::TagParser::new(chunk_body.clone(), define::TagType::AUDIO);
        let tag = parser.parse()?;

        if tag.sound_format == define::sound_format::AAC
            && tag.avc_packet_type == define::aac_packet_type::AAC_SEQHDR
        {
            self.audio_seq = chunk_body.clone();
            self.audio_timestamp = timestamp;
        }

        Ok(())
    }

    pub fn save_video_seq(
        &mut self,
        chunk_body: &mut BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = tag_parser::TagParser::new(chunk_body.clone(), define::TagType::VIDEO);
        let tag = parser.parse()?;

        if tag.frame_type == define::frame_type::KEY_FRAME
            && tag.avc_packet_type == define::avc_packet_type::AVC_SEQHDR
        {
            self.video_seq = chunk_body.clone();
            self.video_timestamp = timestamp;
        }

        Ok(())
    }
}
