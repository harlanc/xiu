use super::define::epat_pid;
use super::errors::MpegTsError;
use super::pmt;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;

use byteorder::BigEndian;

use super::crc32;

#[derive(Debug, Clone)]
pub struct Pat {
    transport_stream_id: u16,
    version_number: u8,     //5bits
    continuity_counter: u8, //s4 bits

    pub pmt_count: u8,
    pub pmt: [pmt::Pmt; 4],
}

impl Pat {
    pub fn default() -> Self {
        Self {
            transport_stream_id: 1,
            version_number: 0,
            continuity_counter: 0,
            pmt_count: 0,
            pmt: [
                pmt::Pmt::default(),
                pmt::Pmt::default(),
                pmt::Pmt::default(),
                pmt::Pmt::default(),
            ],
        }
    }
}
pub struct PatWriter {
    pub bytes_writer: BytesWriter,
}

impl PatWriter {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
        }
    }

    pub fn write(&mut self, pat: Pat) -> Result<(), MpegTsError> {
        /*table id*/
        self.bytes_writer.write_u8(epat_pid::PAT_TID_PAS)?;

        /*section length*/
        let length = pat.pmt_count as u16 * 4 + 5 + 4;
        self.bytes_writer.write_u16::<BigEndian>(0xb000 | length)?;
        /*transport_stream_id*/
        self.bytes_writer
            .write_u16::<BigEndian>(pat.transport_stream_id)?;
        /*version_number*/
        self.bytes_writer
            .write_u8(0xC1 | (pat.version_number << 1))?;

        /*section_number*/
        /*last_section_number*/
        self.bytes_writer.write_u16::<BigEndian>(0x00)?;

        for ele in &pat.pmt {
            /*program number*/
            self.bytes_writer
                .write_u16::<BigEndian>(ele.program_number)?;
            /*PID*/
            self.bytes_writer.write_u16::<BigEndian>(ele.pid as u16)?;
        }

        /*crc32*/
        let crc32_value = crc32::gen_crc32(0xffffffff, self.bytes_writer.extract_current_bytes());
        self.bytes_writer.write_u32::<BigEndian>(crc32_value)?;

        Ok(())
    }
}
