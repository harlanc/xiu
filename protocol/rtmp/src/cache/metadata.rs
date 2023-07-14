use {
    super::errors::MetadataError,
    crate::amf0::{amf0_reader::Amf0Reader, amf0_writer::Amf0Writer, Amf0ValueType},
    bytes::BytesMut,
    bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter},
};
#[derive(Clone)]
pub struct MetaData {
    chunk_body: BytesMut,
    // values: Vec<Amf0ValueType>,
}

impl Default for MetaData {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaData {
    pub fn new() -> Self {
        Self {
            chunk_body: BytesMut::new(),
            //values: Vec::new(),
        }
    }
    //, values: Vec<Amf0ValueType>
    pub fn save(&mut self, body: &BytesMut) {
        if self.is_metadata(body.clone()) {
            self.chunk_body = body.clone();
        }
    }

    //used for the http-flv protocol
    pub fn remove_set_data_frame(&mut self) -> Result<BytesMut, MetadataError> {
        let mut amf_writer: Amf0Writer = Amf0Writer::new(BytesWriter::new());
        amf_writer.write_string(&String::from("@setDataFrame"))?;

        let (_, right) = self.chunk_body.split_at(amf_writer.len());

        Ok(BytesMut::from(right))
    }

    pub fn is_metadata(&mut self, body: BytesMut) -> bool {
        let reader = BytesReader::new(body);
        let result = Amf0Reader::new(reader).read_all();

        let mut values: Vec<Amf0ValueType> = Vec::new();

        match result {
            Ok(v) => {
                values.extend_from_slice(&v[..]);
            }
            Err(_) => return false,
        }

        if values.len() < 2 {
            return false;
        }

        log::info!("metadata: {:?}", values);

        let mut is_metadata = false;

        if let Amf0ValueType::UTF8String(str) = values.remove(0) {
            if str == "@setDataFrame" || str == "onMetaData" {
                is_metadata = true;
            }
        }

        // match values.remove(0) {
        //     Amf0ValueType::UTF8String(str) => {
        //         if str == "@setDataFrame" || str == "onMetaData" {
        //             is_metadata = true;
        //         }
        //     }
        //     _ => {
        //         //return false;
        //     }
        // }
        // match values.remove(0) {
        //     Amf0ValueType::UTF8String(str) => {
        //         if str != "onMetaData" {
        //             //return false;
        //         }
        //     }
        //     _ => {
        //         //return false;
        //     }
        // }

        is_metadata
    }

    pub fn get_chunk_body(&self) -> BytesMut {
        self.chunk_body.clone()
    }
}
