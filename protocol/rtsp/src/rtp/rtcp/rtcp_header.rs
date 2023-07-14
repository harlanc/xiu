use super::errors::RtcpError;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    RC   |   PT          |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone, Default)]
pub struct RtcpHeader {
    pub version: u8,      // 2 bits
    pub padding_flag: u8, // 1 bit
    pub report_count: u8, // 5 bit
    pub payload_type: u8, // 8 bit
    pub length: u16,      // 16 bits
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for RtcpHeader {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut rtcp_header = RtcpHeader::default();

        let byte_1st: u8 = reader.read_u8()?;
        rtcp_header.version = byte_1st >> 6;
        rtcp_header.padding_flag = (byte_1st >> 5) & 0x01;
        rtcp_header.report_count = byte_1st & 0x1F;
        rtcp_header.payload_type = reader.read_u8()?;
        rtcp_header.length = reader.read_u16::<BigEndian>()?;

        Ok(rtcp_header)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpHeader {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let byte_1st: u8 =
            (self.version << 6) | (self.padding_flag << 5) | (self.report_count << 3);

        writer.write_u8(byte_1st)?;
        writer.write_u8(self.payload_type)?;
        writer.write_u16::<BigEndian>(self.length)?;

        Ok(writer.extract_current_bytes())
    }
}
