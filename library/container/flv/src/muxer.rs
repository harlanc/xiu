use super::errors::MuxerError;

use byteorder::BigEndian;
use bytesio::bytes_writer::BytesWriter;

use bytes::BytesMut;

const FLV_HEADER: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x05, //00000101  audio tag  and video tag
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
pub const HEADER_LENGTH: u32 = 11;
pub struct FlvMuxer {
    pub writer: BytesWriter,
}

impl FlvMuxer {
    pub fn new() -> Self {
        Self {
            writer: BytesWriter::new(),
        }
    }

    pub fn write_flv_header(&mut self) -> Result<(), MuxerError> {
        self.writer.write(&FLV_HEADER)?;
        Ok(())
    }

    pub fn write_flv_tag_header(
        &mut self,
        tag_type: u8,
        data_size: u32,
        timestamp: u32,
    ) -> Result<(), MuxerError> {
        //tag type
        self.writer.write_u8(tag_type)?;
        //data size
        self.writer.write_u24::<BigEndian>(data_size)?;
        //timestamp
        self.writer.write_u24::<BigEndian>(timestamp & 0xffffff)?;
        //timestamp extended.
        let timestamp_ext = (timestamp >> 24 & 0xff) as u8;
        self.writer.write_u8(timestamp_ext)?;
        //stream id
        self.writer.write_u24::<BigEndian>(0)?;

        Ok(())
    }

    pub fn write_flv_tag_body(&mut self, body: BytesMut) -> Result<(), MuxerError> {
        self.writer.write(&body[..])?;
        Ok(())
    }

    pub fn write_previous_tag_size(&mut self, size: u32) -> Result<(), MuxerError> {
        self.writer.write_u32::<BigEndian>(size)?;
        Ok(())
    }
}
