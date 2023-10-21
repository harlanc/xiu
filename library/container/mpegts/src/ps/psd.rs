use byteorder::BigEndian;

use crate::define::epes_stream_id::PES_SID_PSD;

use super::errors::MpegPsError;

use {
    bytes::{BufMut, BytesMut},
    bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
};

// Syntax                                       No. of bits Mnemonic
// directory_PES_packet(){
//     packet_start_code_prefix                       24 bslbf
//     directory_stream_id                            8 uimsbf
//     PES_packet_length                              16 uimsbf
//     number_of_access_units                         15 uimsbf
//     marker_bit                                     1 bslbf
//     prev_directory_offset[44..30]                  15 uimsbf
//     marker_bit                                     1 bslbf
//     prev_directory_offset[29..15]                  15 uimsbf
//     marker_bit                                     1 bslbf
//     prev_directory_offset[14..0]                   15 uimsbf
//     marker_bit                                     1 bslbf
//     next_directory_offset[44..30]                  15 uimsbf
//     marker_bit                                     1 bslbf
//     next_directory_offset[29..15]                  15 uimsbf
//     marker_bit                                     1 bslbf
//     next_directory_offset[14..0]                   15 uimsbf
//     marker_bit                                     1 bslbf
//     for (i = 0; i < number_of_access_units; i++) {
//         packet_stream_id                           8 uimsbf
//         PES_header_position_offset_sign            1 tcimsbf
//         PES_header_position_offset[43..30]         14 uimsbf
//         marker_bit                                 1 bslbf
//         PES_header_position_offset[29..15]         15 uimsbf
//         marker_bit                                 1 bslbf
//         PES_header_position_offset[14..0]          15 uimsbf
//         marker_bit                                 1 bslbf
//         reference_offset                           16 uimsbf

//         marker_bit                                 1 bslbf
//         reserved                                   3 bslbf
//         PTS[32..30]                                3 uimsbf
//         marker_bit                                 1 bslbf

//         PTS[29..15]                                15 uimsbf
//         marker_bit                                 1 bslbf

//         PTS[14..0]                                 15 uimsbf
//         marker_bit                                 1 bslbf

//         bytes_to_read[22..8]                       15 uimsbf
//         marker_bit                                 1 bslbf

//         bytes_to_read[7..0]                        8 uimsbf

//         marker_bit                                 1 bslbf
//         intra_coded_indicator                      1 bslbf
//         coding_parameters_indicator                2 bslbf
//         reserved                                   4 bslbf
//     }
// }

#[derive(Default)]
struct AccessUnit {
    packet_stream_id: u8,
    pes_header_position_offset_sign: u8,
    pes_header_position_offset: u64,
    reference_offset: u16,

    pts: u64,
    bytes_to_read: u32,
    intra_coded_indicator: u8,
    coding_parameters_indicator: u8,
}

#[derive(Default)]
pub struct PsProgramStreamDirectory {
    directory_stream_id: u8,
    pes_packet_length: u16,
    number_of_access_units: u16,
    prev_directory_offset: u64,
    next_directory_offset: u64,
    access_units: Vec<AccessUnit>,
}

impl PsProgramStreamDirectory {
    pub fn read(&mut self, bytes_reader: &mut BytesReader) -> Result<(), MpegPsError> {
        let start = bytes_reader.read_bytes(4)?;

        if start.to_vec() != &[0x00, 0x00, 0x01, PES_SID_PSD] {
            return Err(MpegPsError {
                value: super::errors::MpegPsErrorValue::StartCodeNotCorrect,
            });
        }

        self.directory_stream_id = PES_SID_PSD;
        self.pes_packet_length = bytes_reader.read_u16::<BigEndian>()?;
        self.number_of_access_units = bytes_reader.read_u16::<BigEndian>()? >> 1;

        self.prev_directory_offset = bytes_reader.read_u16::<BigEndian>()? as u64 >> 1;
        self.prev_directory_offset = (self.prev_directory_offset << 15)
            | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        self.prev_directory_offset = (self.prev_directory_offset << 15)
            | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

        self.next_directory_offset = bytes_reader.read_u16::<BigEndian>()? as u64 >> 1;
        self.next_directory_offset = (self.next_directory_offset << 15)
            | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        self.next_directory_offset = (self.next_directory_offset << 15)
            | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

        for _ in 0..self.number_of_access_units {
            let packet_stream_id = bytes_reader.read_u8()?;

            let next_2_bytes = bytes_reader.read_u16::<BigEndian>()?;
            let pes_header_position_offset_sign = (next_2_bytes >> 15) as u8;
            //0b11 1111 1111 1111;
            let mut pes_header_position_offset = (next_2_bytes >> 1) as u64 & 0x3FFF;
            pes_header_position_offset = (pes_header_position_offset << 15)
                | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            pes_header_position_offset = (pes_header_position_offset << 15)
                | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

            let reference_offset = bytes_reader.read_u16::<BigEndian>()?;

            let mut pts = (bytes_reader.read_u8()? as u64 >> 1) & 0x07;
            pts = (pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            pts = (pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

            let mut bytes_to_read = bytes_reader.read_u16::<BigEndian>()? as u32 >> 1;
            bytes_to_read = (bytes_to_read << 15) | bytes_reader.read_u8()? as u32;

            let next_byte = bytes_reader.read_u8()?;
            let intra_coded_indicator = (next_byte >> 6) & 0x01;
            let coding_parameters_indicator = (next_byte >> 4) & 0x03;

            self.access_units.push(AccessUnit {
                packet_stream_id,
                pes_header_position_offset_sign,
                pes_header_position_offset,
                reference_offset,
                pts,
                bytes_to_read,
                intra_coded_indicator,
                coding_parameters_indicator,
            });
        }

        Ok(())
    }
}
