use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    ST   |   PT=APP=204  |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           SSRC/CSRC                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                          name (ASCII)                         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                   application-dependent data                ...
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone, Default)]
pub struct RtcpApp {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub name: BytesMut,
    pub app_data: BytesMut,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpApp {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut rtcp_app = RtcpApp::default();
        rtcp_app.header = RtcpHeader::unmarshal(&mut reader)?;

        rtcp_app.ssrc = reader.read_u32::<BigEndian>()?;
        rtcp_app.name = reader.read_bytes(4)?;
        rtcp_app.app_data = reader.read_bytes(rtcp_app.header.length as usize * 4)?;

        Ok(rtcp_app)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpApp {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write(&self.name[..])?;
        writer.write(&self.app_data[..])?;

        Ok(writer.extract_current_bytes())
    }
}
