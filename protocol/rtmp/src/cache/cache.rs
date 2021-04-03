use super::metadata;
use crate::amf0::Amf0ValueType;
use crate::messages;
use bytes::BytesMut;
use flv::tag_parser;
use flv::define::TagType;


pub struct Cache {
    meta_data: metadata::MetaData,
    video_seq: BytesMut,
    audio_seq: BytesMut,
}

impl Cache {
    fn new() -> Self {
        Self {
            meta_data: metadata::MetaData::default(),
            video_seq: BytesMut::new(),
            audio_seq: BytesMut::new(),
        }
    }

    pub fn save_metadata(&mut self, chunk_body: &mut BytesMut, values: &mut Vec<Amf0ValueType>) {
        self.meta_data.save(chunk_body, values);
    }

    pub fn save_audio_seq(&mut self, chunk_body: &mut BytesMut) {

        let parser = tag_parser::TagParser::new(chunk_body,TagType::AUDIO);
        let tag = parser.parse()?;


    }

    pub fn save_video_seq(&mut self, chunk_body: &mut BytesMut) {}
}
