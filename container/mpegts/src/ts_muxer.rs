use super::define::epat_pid;
use super::define::ts;
use super::errors::MpegTsError;
use super::pat;
use super::pmt;
use bytes::BytesMut;
use networkio::bytes_writer::BytesWriter;

pub struct TsWriter {
    bytes_writer: BytesWriter,
    pat_continuity_counter: u8,
    pmt_continuity_counter: u8,
}

impl TsWriter {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            pat_continuity_counter: 0,
            pmt_continuity_counter: 0,
        }
    }

    pub fn write(&mut self, pat_data: pat::Pat) -> Result<(), MpegTsError> {
        let mut pat_writer = pat::PatWriter::new();

        pat_writer.write(pat_data.clone())?;
        self.write_section_header(
            epat_pid::PAT_TID_PAS,
            pat_writer.bytes_writer.extract_current_bytes(),
        )?;

        let mut pmt_writer = pmt::PmtWriter::new();
        for pmt_data in &pat_data.pmt {
            pmt_writer.write(pmt_data)?;
            self.write_section_header(
                epat_pid::PAT_TID_PMS,
                pmt_writer.bytes_writer.extract_current_bytes(),
            )?;
        }

        Ok(())
    }

    pub fn write_section_header(&mut self, pid: u8, payload: BytesMut) -> Result<(), MpegTsError> {
        self.bytes_writer.write_u8(pid)?;
        self.bytes_writer.write_u8(0x40 | ((pid >> 8) & 0x1F))?;
        self.bytes_writer.write_u8(pid & 0xFF)?;

        match pid {
            epat_pid::PAT_TID_PAS => {
                self.pat_continuity_counter = (self.pat_continuity_counter + 1) % 16;
            }
            epat_pid::PAT_TID_PMS => {
                self.pmt_continuity_counter = (self.pat_continuity_counter + 1) % 16;
            }

            _ => {}
        }

        self.bytes_writer.write_u8(0x00)?;
        self.bytes_writer.write(&payload)?;

        let cur_size = self.bytes_writer.extract_current_bytes().len();
        let left_size = ts::TS_PACKET_SIZE - cur_size as u8;

        for _ in 0..left_size {
            self.bytes_writer.write_u8(0xFF)?;
        }
        Ok(())
    }
}
