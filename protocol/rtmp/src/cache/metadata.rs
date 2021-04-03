use crate::amf0::Amf0ValueType;
use bytes::BytesMut;
use netio::bytes_reader::BytesReader;
use netio::bytes_writer::BytesWriter;

pub struct MetaData {
    chunk_body: BytesMut,
    values: Vec<Amf0ValueType>,
}

impl MetaData {
    pub fn default() -> Self {
        Self {
            chunk_body: BytesMut::new(),
            values: Vec::new(),
        }
    }
    pub fn save(&mut self, body: &mut BytesMut, values: &mut Vec<Amf0ValueType>) {
        if self.is_metadata(body, values) {
            self.chunk_body = body.clone();
            self.values = values.clone();
        }
    }

    pub fn is_metadata(&mut self, body: &mut BytesMut, values: &mut Vec<Amf0ValueType>) -> bool {
        loop {
            if values.len() < 2 {
                return false;
            }

            match values.remove(0) {
                Amf0ValueType::UTF8String(str) => {
                    if str != "@setDataFrame" {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            }

            match values.remove(0) {
                Amf0ValueType::UTF8String(str) => {
                    if str != "onMetaData" {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            }
            break;
        }

        return true;
    }

    pub fn get_chunk_body(&self) -> BytesMut {
        return self.chunk_body.clone();
    }
}
