use byteorder::BigEndian;

use crate::define::epes_stream_id::PES_SID_SYS;

use super::errors::MpegPsError;

use {
    bytes::{BufMut, BytesMut},
    bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
};
pub struct PsStream {
    stream_id: u8,
    stream_id_extension: u8,
    buffer_bound_scale: u8,
    buffer_size_bound: u16,
}

pub struct PsSystemHeader {
    header_length: u16,
    rate_bound: u32,
    audio_bound: u8,
    fixed_flag: u8,
    csps_flag: u8,
    system_audio_lock_flag: u8,
    system_video_lock_flag: u8,
    video_bound: u8,
    packet_rate_restriction_flag: u8,
    streams: Vec<PsStream>,
}

impl PsSystemHeader {
    //T-REC-H.222.0-201703-S!!PDF-E.pdf Table 2-40 P66
    // system_header () {
    //     system_header_start_code 				32 bslbf
    //     header_length 							16 uimsbf
    //     marker_bit 								1 bslbf
    //     rate_bound 								22 uimsbf
    //     marker_bit 								1 bslbf
    //     audio_bound 							    6 uimsbf
    //     fixed_flag 								1 bslbf
    //     CSPS_flag 								1 bslbf
    //     system_audio_lock_flag 					1 bslbf
    //     system_video_lock_flag 					1 bslbf
    //     marker_bit								1 bslbf
    //     video_bound 							    5 uimsbf
    //     packet_rate_restriction_flag			    1 bslbf
    //     reserved_bits 						    7 bslbf
    //     while (nextbits () == '1') {
    //         stream_id 							8 uimsbf
    //         if (stream_id == '1011 0111') {
    //             '11' 							2 bslbf
    //             '000 0000' 						7 bslbf
    //             stream_id_extension 			    7 uimsbf
    //             '1011 0110' 				    	8 bslbf
    //             '11' 							2 bslbf
    //             P-STD_buffer_bound_scale 		1 bslbf
    //             P-STD_buffer_size_bound 		    13 uimsbf
    //         }
    //         else {
    //             '11' 							2 bslbf
    //             P-STD_buffer_bound_scale 		1 bslbf
    //             P-STD_buffer_size_bound 		    13 uimsbf
    //         }
    //     }
    // }
    pub fn read(&mut self, payload: BytesMut) -> Result<(), MpegPsError> {
        let mut bytes_reader = BytesReader::new(payload);
        let start = bytes_reader.read_bytes(4)?;

        if start.to_vec() != &[0x00, 0x00, 0x01, PES_SID_SYS] {
            return Err(MpegPsError {
                value: super::errors::MpegPsErrorValue::StartCodeNotCorrect,
            });
        }

        self.header_length = bytes_reader.read_u16::<BigEndian>()?;
        self.rate_bound = (bytes_reader.read_u24::<BigEndian>()? & 0x7FFFFF) >> 1;

        let byte_10 = bytes_reader.read_u8()?;
        self.audio_bound = byte_10 >> 2;
        self.fixed_flag = (byte_10 >> 1) & 0x01;
        self.csps_flag = byte_10 & 0x01;

        let byte_11 = bytes_reader.read_u8()?;
        self.system_audio_lock_flag = byte_11 >> 7;
        self.system_video_lock_flag = (byte_11 >> 6) & 0x01;
        self.video_bound = byte_11 & 0x1F;

        let byte_12 = bytes_reader.read_u8()?;
        self.packet_rate_restriction_flag = byte_12 >> 7;

        while bytes_reader.len() > 0 && (bytes_reader.advance_u8()? >> 7) == 0x01 {
            let stream_id = bytes_reader.read_u8()?;

            let stream_id_extension = if stream_id == 0xB7 {
                let next_byte = bytes_reader.read_u8()?;
                assert!(next_byte >> 6 == 0b11);
                assert!(next_byte & 0x3F == 0b0);

                let next_byte_2 = bytes_reader.read_u8()?;
                assert!(next_byte_2 >> 7 == 0b0);
                let stream_id_extension = next_byte_2 & 0x7F;

                let next_byte_3 = bytes_reader.read_u8()?;
                assert!(next_byte_3 == 0b10110110);
                stream_id_extension
            } else {
                0
            };

            let next_2bytes = bytes_reader.read_u16::<BigEndian>()?;
            assert!(next_2bytes >> 14 == 0b11);
            let buffer_bound_scale = (next_2bytes >> 13) as u8 & 0x01;
            let buffer_size_bound = next_2bytes & 0x1FFF;

            self.streams.push(PsStream {
                stream_id,
                stream_id_extension,
                buffer_bound_scale,
                buffer_size_bound,
            });
        }

        Ok(())
    }
}

mod tests {
    use byteorder::BigEndian;

    use {
        bytes::{BufMut, BytesMut},
        bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
    };

    #[test]
    pub fn test_bytes_reader() {
        let v = [0xFF, 0x01, 0x02];
        let mut b = BytesMut::new();
        b.extend_from_slice(&v);
        let mut reader1 = BytesReader::new(b.clone());

        let mut read2 = BytesReader::new(b);

        let mut bits_reader = BitsReader::new(reader1);

        println!("{}", bits_reader.read_bit().unwrap());

        println!("{}", bits_reader.read_n_bits(22).unwrap());
        println!("{}", bits_reader.read_bit().unwrap());

        let aa = read2.read_u24::<BigEndian>().unwrap();

        println!("{}", (aa & 0x7FFFFF) >> 1);
    }
}
