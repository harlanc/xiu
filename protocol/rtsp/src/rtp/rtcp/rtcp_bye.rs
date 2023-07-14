use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
//  	  0                   1                   2                   3
//  	  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |V=2|P|    SC   |   PT=BYE=203  |             length            |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |                           SSRC/CSRC                           |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     :                              ...                              :
// 	     +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// (opt) |     length    |            reason for leaving     ...
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone, Default)]
pub struct RtcpBye {
    pub header: RtcpHeader,
    pub ssrss: Vec<u32>,
    pub length: u8,
    pub reason: BytesMut,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpBye {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut rtcp_bye = RtcpBye::default();
        rtcp_bye.header = RtcpHeader::unmarshal(&mut reader)?;

        for _ in 0..rtcp_bye.header.report_count {
            let ssrc = reader.read_u32::<BigEndian>()?;
            rtcp_bye.ssrss.push(ssrc);
        }

        rtcp_bye.length = reader.read_u8()?;
        rtcp_bye.reason = reader.read_bytes(rtcp_bye.length as usize)?;

        Ok(rtcp_bye)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpBye {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        for ssrc in &self.ssrss {
            writer.write_u32::<BigEndian>(*ssrc)?;
        }

        writer.write_u8(self.length)?;
        writer.write(&self.reason[..])?;

        Ok(writer.extract_current_bytes())
    }
}
