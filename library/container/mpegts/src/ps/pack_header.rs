use byteorder::BigEndian;

use crate::define::epes_stream_id::PES_SID_START;

use super::errors::MpegPsError;

use {
    bytes::{BufMut, BytesMut},
    bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
};

enum MpegType {
    Mpeg1,
    Mpeg2,
}
pub struct PsPackHeader {
    mpeg_type: MpegType,
    system_clock_reference_base: u64,
    system_clock_reference_extension: u16,
    program_mux_rate: u32,
    pack_stuffing_length: u8,
}

impl PsPackHeader {
    pub fn read(&mut self, payload: BytesMut) -> Result<(), MpegPsError> {
        let mut bytes_reader = BytesReader::new(payload);
        let start = bytes_reader.read_bytes(4)?;

        if start.to_vec() != &[0x00, 0x00, 0x01, PES_SID_START] {
            return Err(MpegPsError {
                value: super::errors::MpegPsErrorValue::StartCodeNotCorrect,
            });
        }
        let next_byte = bytes_reader.advance_u8()?;

        let mut bits_reader = BitsReader::new(bytes_reader);

        //mpeg1
        if (next_byte >> 4) == 0b0010 {
            self.mpeg_type = MpegType::Mpeg1;
            bits_reader.read_n_bits(4)?;
            self.system_clock_reference_base = bits_reader.read_n_bits(3)?;
            bits_reader.read_bit()?;

            self.system_clock_reference_base =
                self.system_clock_reference_base << 15 | bits_reader.read_n_bits(15)?;
            bits_reader.read_bit()?;

            self.system_clock_reference_base =
                self.system_clock_reference_base << 15 | bits_reader.read_n_bits(15)?;
            bits_reader.read_bit()?;

            self.system_clock_reference_extension = 1;
            self.program_mux_rate = bits_reader.read_n_bits(7)? as u32;
            bits_reader.read_bit()?;

            self.program_mux_rate =
                self.program_mux_rate << 15 | bits_reader.read_n_bits(15)? as u32;
            bits_reader.read_bit()?;
        }
        //mpeg2
        else if (next_byte >> 6) == 0b01 {
            self.mpeg_type = MpegType::Mpeg2;
            bits_reader.read_n_bits(2)?;
            self.system_clock_reference_base = bits_reader.read_n_bits(3)?;
            bits_reader.read_bit()?;
            self.system_clock_reference_base =
                self.system_clock_reference_base << 15 | bits_reader.read_n_bits(15)?;
            bits_reader.read_bit()?;
            self.system_clock_reference_base =
                self.system_clock_reference_base << 15 | bits_reader.read_n_bits(15)?;
            bits_reader.read_bit()?;
            self.system_clock_reference_extension = bits_reader.read_n_bits(9)? as u16;
            bits_reader.read_bit()?;

            self.program_mux_rate = bits_reader.read_n_bits(22)? as u32;
            bits_reader.read_n_bits(7)?;
            self.pack_stuffing_length = bits_reader.read_n_bits(3)? as u8;
        }

        Ok(())
    }
}
