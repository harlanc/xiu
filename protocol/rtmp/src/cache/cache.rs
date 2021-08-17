use {
    super::{errors::CacheError, metadata},
    crate::channels::define::ChannelData,
    bytes::BytesMut,
    xflv::{define, demuxer_tag},
};

pub struct Cache {
    metadata: metadata::MetaData,
    metadata_timestamp: u32,
    video_seq: BytesMut,
    video_timestamp: u32,
    audio_seq: BytesMut,
    audio_timestamp: u32,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            metadata: metadata::MetaData::default(),
            metadata_timestamp: 0,
            video_seq: BytesMut::new(),
            video_timestamp: 0,
            audio_seq: BytesMut::new(),
            audio_timestamp: 0,
        }
    }

    // async fn write_amf_data_chunk(&mut self) -> Result<(), CacheError> {
    //     let data = self.metadata.get_chunk_body();
    //     let mut chunk_info = ChunkInfo::new(
    //         chunk_define::csid_type::DATA_AMF0_AMF3,
    //         chunk_define::chunk_type::TYPE_0,
    //         0,
    //         data.len() as u32,
    //         messages_define::msg_type_id::DATA_AMF0,
    //         0,
    //         data,
    //     );

    //     self.packetizer.write_chunk(&mut chunk_info).await?;
    //     Ok(())
    // }

    // async fn write_video_seq_chunk(&mut self) -> Result<(), CacheError> {
    //     let mut chunk_info = ChunkInfo::new(
    //         chunk_define::csid_type::VIDEO,
    //         chunk_define::chunk_type::TYPE_0,
    //         self.video_timestamp,
    //         self.video_seq.len() as u32,
    //         messages_define::msg_type_id::VIDEO,
    //         0,
    //         self.video_seq.clone(),
    //     );

    //     self.packetizer.write_chunk(&mut chunk_info).await?;
    //     Ok(())
    // }

    // async fn write_audio_seq_chunk(&mut self) -> Result<(), CacheError> {
    //     let mut chunk_info = ChunkInfo::new(
    //         chunk_define::csid_type::AUDIO,
    //         chunk_define::chunk_type::TYPE_0,
    //         self.audio_timestamp,
    //         self.audio_seq.len() as u32,
    //         messages_define::msg_type_id::AUDIO,
    //         0,
    //         self.audio_seq.clone(),
    //     );

    //     self.packetizer.write_chunk(&mut chunk_info).await?;
    //     Ok(())
    //}

    // pub async fn send(&mut self) -> Result<(), CacheError> {
    //     self.write_amf_data_chunk().await?;
    //     self.write_video_seq_chunk().await?;
    //     self.write_audio_seq_chunk().await?;
    //     Ok(())
    // }

    //, values: Vec<Amf0ValueType>
    pub fn save_metadata(&mut self, chunk_body: BytesMut, timestamp: u32) {
        self.metadata.save(chunk_body);
        self.metadata_timestamp = timestamp;
    }

    pub fn get_metadata(&self) -> Option<ChannelData> {
        let data = self.metadata.get_chunk_body();
        if data.len() > 0 {
            Some(ChannelData::MetaData {
                timestamp: self.metadata_timestamp,
                data,
            })
        } else {
            None
        }
    }

    pub fn save_audio_seq(
        &mut self,
        chunk_body: BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = demuxer_tag::AudioTagHeaderDemuxer::new(chunk_body.clone());
        let tag = parser.parse_tag_header()?;

        if tag.sound_format == define::sound_format::AAC
            && tag.aac_packet_type == define::aac_packet_type::AAC_SEQHDR
        {
            self.audio_seq = chunk_body;
            self.audio_timestamp = timestamp;
        }

        Ok(())
    }

    pub fn get_audio_seq(&self) -> Option<ChannelData> {
        if self.audio_seq.len() > 0 {
            return Some(ChannelData::Audio {
                timestamp: self.audio_timestamp,
                data: self.audio_seq.clone(),
            });
        }
        None
    }

    pub fn get_video_seq(&self) -> Option<ChannelData> {
        if self.video_seq.len() > 0 {
            return Some(ChannelData::Video {
                timestamp: self.video_timestamp,
                data: self.video_seq.clone(),
            });
        }
        None
    }

    pub fn save_video_seq(
        &mut self,
        chunk_body: BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let mut parser = demuxer_tag::VideoTagHeaderDemuxer::new(chunk_body.clone());
        let tag = parser.parse_tag_header()?;

        if tag.frame_type == define::frame_type::KEY_FRAME
            && tag.avc_packet_type == define::avc_packet_type::AVC_SEQHDR
        {
            self.video_seq = chunk_body;
            self.video_timestamp = timestamp;
        }

        Ok(())
    }
}
