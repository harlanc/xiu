use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use super::rtcp_rr::ReportBlock;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// header 	|V=2|P|    RC   |   PT=SR=200   |             length            |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         SSRC of sender                        |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// sender 	|              NTP timestamp, most significant word             |
// info   	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|             NTP timestamp, least significant word             |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         RTP timestamp                         |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                     sender's packet count                     |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                      sender's octet count                     |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// report 	|                 SSRC_1 (SSRC of first source)                 |
// block  	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 1    	| fraction lost |       cumulative number of packets lost       |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|           extended highest sequence number received           |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                      interarrival jitter                      |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         last SR (LSR)                         |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                   delay since last SR (DLSR)                  |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// report 	|                 SSRC_2 (SSRC of second source)                |
// block  	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 2    	:                               ...                             :
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// 			|                  profile-specific extensions                  |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone, Default)]
pub struct RtcpSenderReport {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub ntp: u64,
    rtp_timestamp: u32,
    sender_packet_count: u32,
    sender_octet_count: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for RtcpSenderReport {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut sender_report = RtcpSenderReport::default();
        sender_report.header = RtcpHeader::unmarshal(reader)?;

        sender_report.ssrc = reader.read_u32::<BigEndian>()?;
        sender_report.ntp = reader.read_u64::<BigEndian>()?;
        sender_report.rtp_timestamp = reader.read_u32::<BigEndian>()?;
        sender_report.sender_packet_count = reader.read_u32::<BigEndian>()?;
        sender_report.sender_octet_count = reader.read_u32::<BigEndian>()?;

        for _ in 0..sender_report.header.report_count {
            let report_block = ReportBlock::unmarshal(reader)?;
            sender_report.report_blocks.push(report_block);
        }

        Ok(sender_report)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpSenderReport {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u64::<BigEndian>(self.ntp)?;
        writer.write_u32::<BigEndian>(self.rtp_timestamp)?;
        writer.write_u32::<BigEndian>(self.sender_packet_count)?;
        writer.write_u32::<BigEndian>(self.sender_octet_count)?;

        for report_block in &self.report_blocks {
            let data = report_block.marshal()?;
            writer.write(&data[..])?;
        }

        Ok(writer.extract_current_bytes())
    }
}
