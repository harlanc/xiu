use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

#[derive(Debug, Clone, Default)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumutlative_num_of_packets_lost: u32,
    pub extended_highest_seq_number: u32,
    pub jitter: u32,
    pub lsr: u32,
    pub dlsr: u32,
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for ReportBlock {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        Ok(ReportBlock {
            ssrc: reader.read_u32::<BigEndian>()?,
            fraction_lost: reader.read_u8()?,
            cumutlative_num_of_packets_lost: reader.read_u24::<BigEndian>()?,
            extended_highest_seq_number: reader.read_u32::<BigEndian>()?,
            jitter: reader.read_u32::<BigEndian>()?,
            lsr: reader.read_u32::<BigEndian>()?,
            dlsr: reader.read_u32::<BigEndian>()?,
        })
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for ReportBlock {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8(self.fraction_lost)?;
        writer.write_u24::<BigEndian>(self.cumutlative_num_of_packets_lost)?;
        writer.write_u32::<BigEndian>(self.extended_highest_seq_number)?;
        writer.write_u32::<BigEndian>(self.jitter)?;
        writer.write_u32::<BigEndian>(self.lsr)?;
        writer.write_u32::<BigEndian>(self.dlsr)?;

        Ok(writer.extract_current_bytes())
    }
}

#[derive(Debug, Clone, Default)]
pub struct RtcpReceiverReport {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpReceiverReport {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut receiver_report = RtcpReceiverReport {
            header: RtcpHeader::unmarshal(&mut reader)?,
            ssrc: reader.read_u32::<BigEndian>()?,
            ..Default::default()
        };

        for _ in 0..receiver_report.header.report_count {
            let report_block = ReportBlock::unmarshal(&mut reader)?;
            receiver_report.report_blocks.push(report_block);
        }

        Ok(receiver_report)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpReceiverReport {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        for report_block in &self.report_blocks {
            let data = report_block.marshal()?;
            writer.write(&data[..])?;
        }

        Ok(writer.extract_current_bytes())
    }
}
