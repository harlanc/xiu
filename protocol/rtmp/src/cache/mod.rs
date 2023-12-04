pub mod errors;
pub mod gop;
pub mod metadata;

use {
    self::gop::Gops,
    bytes::BytesMut,
    bytesio::bytes_reader::BytesReader,
    errors::CacheError,
    gop::Gop,
    std::collections::VecDeque,
    streamhub::define::FrameData,
    streamhub::statistics::avstatistics::AvStatistics,
    streamhub::stream::StreamIdentifier,
    xflv::{
        define,
        flv_tag_header::{AudioTagHeader, VideoTagHeader},
        mpeg4_aac::Mpeg4AacProcessor,
        mpeg4_avc::Mpeg4AvcProcessor,
        Unmarshal,
    },
};

// #[derive(Clone)]
pub struct Cache {
    metadata: metadata::MetaData,
    metadata_timestamp: u32,
    video_seq: BytesMut,
    video_timestamp: u32,
    audio_seq: BytesMut,
    audio_timestamp: u32,
    gops: Gops,
    pub av_statistics: AvStatistics,
}

impl Drop for Cache {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        self.av_statistics.sender.send(true);
    }
}

impl Cache {
    pub fn new(app_name: String, stream_name: String, gop_num: usize) -> Self {
        let identifier = StreamIdentifier::Rtmp {
            app_name,
            stream_name,
        };
        let mut cache = Cache {
            metadata: metadata::MetaData::new(),
            metadata_timestamp: 0,
            video_seq: BytesMut::new(),
            video_timestamp: 0,
            audio_seq: BytesMut::new(),
            audio_timestamp: 0,
            gops: Gops::new(gop_num),
            av_statistics: AvStatistics::new(identifier),
        };
        cache.av_statistics.start();
        cache
    }

    //, values: Vec<Amf0ValueType>
    pub fn save_metadata(&mut self, chunk_body: &BytesMut, timestamp: u32) {
        self.metadata.save(chunk_body);
        self.metadata_timestamp = timestamp;
    }

    pub fn get_metadata(&self) -> Option<FrameData> {
        let data = self.metadata.get_chunk_body();
        if !data.is_empty() {
            Some(FrameData::MetaData {
                timestamp: self.metadata_timestamp,
                data,
            })
        } else {
            None
        }
    }
    //save audio gops and sequence header information
    pub async fn save_audio_data(
        &mut self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        let channel_data = FrameData::Audio {
            timestamp,
            data: chunk_body.clone(),
        };
        self.gops.save_frame_data(channel_data, false);

        let mut reader = BytesReader::new(chunk_body.clone());
        let tag_header = AudioTagHeader::unmarshal(&mut reader)?;

        if tag_header.sound_format == define::SoundFormat::AAC as u8
            && tag_header.aac_packet_type == define::aac_packet_type::AAC_SEQHDR
        {
            self.audio_seq = chunk_body.clone();
            self.audio_timestamp = timestamp;

            let mut aac_processor = Mpeg4AacProcessor::default();
            let aac = aac_processor
                .extend_data(reader.extract_remaining_bytes())
                .audio_specific_config_load()?;
            self.av_statistics
                .notify_audio_codec_info(&aac.mpeg4_aac)
                .await;
        }

        self.av_statistics
            .notify_audio_statistics_info(chunk_body.len(), tag_header.aac_packet_type)
            .await;

        Ok(())
    }

    pub fn get_audio_seq(&self) -> Option<FrameData> {
        if !self.audio_seq.is_empty() {
            return Some(FrameData::Audio {
                timestamp: self.audio_timestamp,
                data: self.audio_seq.clone(),
            });
        }
        None
    }

    pub fn get_video_seq(&self) -> Option<FrameData> {
        if !self.video_seq.is_empty() {
            return Some(FrameData::Video {
                timestamp: self.video_timestamp,
                data: self.video_seq.clone(),
            });
        }
        None
    }
    //save video gops and sequence header information
    pub async fn save_video_data(
        &mut self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        log::info!("save video data");
        let channel_data = FrameData::Video {
            timestamp,
            data: chunk_body.clone(),
        };

        let mut reader = BytesReader::new(chunk_body.clone());
        let tag_header = VideoTagHeader::unmarshal(&mut reader)?;

        let is_key_frame = tag_header.frame_type == define::frame_type::KEY_FRAME;
        self.gops.save_frame_data(channel_data, is_key_frame);

        if is_key_frame && tag_header.avc_packet_type == define::avc_packet_type::AVC_SEQHDR {
            log::info!("save seq header");
            let mut avc_processor = Mpeg4AvcProcessor::default();
            avc_processor.decoder_configuration_record_load(&mut reader)?;

            self.av_statistics
                .notify_video_codec_info(&avc_processor.mpeg4_avc)
                .await;

            self.video_seq = chunk_body.clone();
            self.video_timestamp = timestamp;
        }

        self.av_statistics
            .notify_video_statistics_info(chunk_body.len(), is_key_frame)
            .await;

        Ok(())
    }

    pub fn get_gops_data(&self) -> Option<VecDeque<Gop>> {
        if self.gops.setted() {
            Some(self.gops.get_gops())
        } else {
            None
        }
    }
}
