use byteorder::BigEndian;

use crate::define::epes_stream_id::PES_SID_START;

use super::errors::MpegPsError;

use {
    bytes::{BufMut, BytesMut},
    bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
};

#[derive(Default)]
enum MpegType {
    Mpeg1,
    Mpeg2,
    #[default]
    Unknown,
}
#[derive(Default)]
pub struct PsPackHeader {
    mpeg_type: MpegType,
    system_clock_reference_base: u64,
    system_clock_reference_extension: u16,
    program_mux_rate: u32,
    pack_stuffing_length: u8,
}

impl PsPackHeader {
    pub fn parse(&mut self, bytes_reader: &mut BytesReader) -> Result<(), MpegPsError> {
        let start = bytes_reader.read_bytes(4)?;

        if start.to_vec() != &[0x00, 0x00, 0x01, PES_SID_START] {
            return Err(MpegPsError {
                value: super::errors::MpegPsErrorValue::StartCodeNotCorrect,
            });
        }
        let byte_5 = bytes_reader.read_u8()?;

        //mpeg1
        if (byte_5 >> 4) == 0b0010 {
            self.mpeg_type = MpegType::Mpeg1;

            self.system_clock_reference_base = (byte_5 as u64 >> 1) & 0x07;
            self.system_clock_reference_base = (self.system_clock_reference_base << 15)
                | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.system_clock_reference_base = (self.system_clock_reference_base << 15)
                | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

            self.system_clock_reference_extension = 1;

            let byte_10 = bytes_reader.read_u8()?;
            self.program_mux_rate = (byte_10 as u32) >> 1;
            self.program_mux_rate =
                (self.program_mux_rate << 15) | (bytes_reader.read_u16::<BigEndian>()? as u32 >> 1);
        }
        //mpeg2
        else if (byte_5 >> 6) == 0b01 {
            self.mpeg_type = MpegType::Mpeg2;

            self.system_clock_reference_base = (byte_5 as u64 >> 3) & 0x07;
            self.system_clock_reference_base =
                (self.system_clock_reference_base << 2) | (byte_5 as u64 & 0x03);
            let next_two_bytes = bytes_reader.read_u16::<BigEndian>()?;
            self.system_clock_reference_base =
                (self.system_clock_reference_base << 13) | (next_two_bytes as u64 >> 3);
            self.system_clock_reference_base =
                (self.system_clock_reference_base << 2) | (next_two_bytes as u64 & 0x03);
            let next_two_bytes_2 = bytes_reader.read_u16::<BigEndian>()?;
            self.system_clock_reference_base =
                (self.system_clock_reference_base << 13) | (next_two_bytes_2 as u64 >> 3);

            self.system_clock_reference_extension = next_two_bytes_2 & 0x03;
            self.system_clock_reference_extension = (self.system_clock_reference_extension << 7)
                | (bytes_reader.read_u8()? as u16 >> 1);

            self.program_mux_rate = bytes_reader.read_u24::<BigEndian>()? >> 2; //bits_reader.read_n_bits(22)? as u32;
            self.pack_stuffing_length = bytes_reader.read_u8()? & 0x07;
        }

        Ok(())
    }
}
