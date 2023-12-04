use crate::ps::errors::{MpegPsError, MpegPsErrorValue};
use byteorder::BigEndian;
use {
    super::define, super::errors::MpegError, bytes::BytesMut, bytesio::bytes_reader::BytesReader,
    bytesio::bytes_writer::BytesWriter,
};

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Pes {
    pub stream_id: u8,                 //8
    pub pes_packet_length: u16,        //16
    pub pes_scrambling_control: u8,    //2
    pub pes_priority: u8,              //2
    pub data_alignment_indicator: u8,  //1
    pub copyright: u8,                 //1
    pub original_or_copy: u8,          //1
    pub pts_dts_flags: u8,             //2
    pub escr_flag: u8,                 //1
    pub es_rate_flag: u8,              //1
    pub dsm_trick_mode_flag: u8,       //1
    pub additional_copy_info_flag: u8, //1

    pub pes_crc_flag: u8,           //1
    pub pes_extension_flag: u8,     //1
    pub pes_header_data_length: u8, //8

    pub pts: u64,
    pub dts: u64,
    escr_base: u64,
    escr_extension: u32,
    es_rate: u32,

    pub trick_mode_control: u8,
    pub trick_value: u8,
    pub additional_copy_info: u8,
    pub previous_pes_packet_crc: u16,
    pub payload: BytesMut,

    pub pid: u16,
    pub codec_id: u8,
    pub continuity_counter: u8,
}

impl Pes {
    //  T-REC-H.222.0-201703-S!!PDF-E.pdf Table 2-21 P37
    // PES_packet() {
    //     packet_start_code_prefix 24 bslbf
    //     stream_id 8 uimsbf
    //     PES_packet_length 16 uimsbf

    //     if (stream_id != program_stream_map
    //     && stream_id != padding_stream
    //     && stream_id != private_stream_2
    //     && stream_id != ECM
    //     && stream_id != EMM
    //     && stream_id != program_stream_directory
    //     && stream_id != DSMCC_stream
    //     && stream_id != ITU-T Rec. H.222.1 type E stream) {
    //         '10' 2 bslbf
    //         PES_scrambling_control 2 bslbf
    //         PES_priority 1 bslbf
    //         data_alignment_indicator 1 bslbf
    //         copyright 1 bslbf
    //         original_or_copy 1 bslbf
    //         PTS_DTS_flags 2 bslbf
    //         ESCR_flag 1 bslbf
    //         ES_rate_flag 1 bslbf
    //         DSM_trick_mode_flag 1 bslbf
    //         additional_copy_info_flag 1 bslbf
    //         PES_CRC_flag 1 bslbf
    //         PES_extension_flag 1 bslbf
    //         PES_header_data_length 8 uimsbf

    //         if (PTS_DTS_flags == '10') {
    //             '0010' 4 bslbf
    //             PTS [32..30] 3 bslbf
    //             marker_bit 1 bslbf
    //             PTS [29..15] 15 bslbf
    //             marker_bit 1 bslbf
    //             PTS [14..0] 15 bslbf
    //             marker_bit 1 bslbf
    //         }

    //         if (PTS_DTS_flags == '11') {
    //             '0011' 4 bslbf
    //             PTS [32..30] 3 bslbf
    //             marker_bit 1 bslbf
    //             PTS [29..15] 15 bslbf
    //             marker_bit 1 bslbf
    //             PTS [14..0] 15 bslbf
    //             marker_bit 1 bslbf
    //             '0001' 4 bslbf
    //             DTS [32..30] 3 bslbf
    //             marker_bit 1 bslbf
    //             DTS [29..15] 15 bslbf
    //             marker_bit 1 bslbf
    //             DTS [14..0] 15 bslbf
    //             marker_bit 1 bslbf
    //         }

    //         if (ESCR_flag == '1') {
    //             reserved 2 bslbf
    //             ESCR_base[32..30] 3 bslbf
    //             marker_bit 1 bslbf
    //             ESCR_base[29..15] 15 bslbf
    //             marker_bit 1 bslbf
    //             ESCR_base[14..0] 15 bslbf
    //             marker_bit 1 bslbf
    //             ESCR_extension 9 uimsbf
    //             marker_bit 1 bslbf
    //         }

    //         if (ES_rate_flag == '1') {
    //             marker_bit 1 bslbf
    //             ES_rate 22 uimsbf
    //             marker_bit 1 bslbf
    //         }

    //         if (DSM_trick_mode_flag == '1') {
    //             trick_mode_control 3 uimsbf
    //             if ( trick_mode_control == fast_forward ) {
    //                 field_id 2 bslbf
    //                 intra_slice_refresh 1 bslbf
    //                 frequency_truncation 2 bslbf
    //             }
    //         else if ( trick_mode_control == slow_motion ) {
    //             rep_cntrl 5 uimsbf
    //         }
    //         else if ( trick_mode_control == freeze_frame ) {
    //             field_id 2 uimsbf
    //             reserved 3 bslbf
    //         }
    //         else if ( trick_mode_control == fast_reverse ) {
    //             field_id 2 bslbf
    //             intra_slice_refresh 1 bslbf
    //             frequency_truncation 2 bslbf
    //         else if ( trick_mode_control == slow_reverse ) {
    //             rep_cntrl 5 uimsbf
    //         }
    //         else
    //             reserved 5 bslbf
    //         }

    //         if ( additional_copy_info_flag == '1') {
    //             marker_bit 1 bslbf
    //             additional_copy_info 7 bslbf
    //         }

    //         if ( PES_CRC_flag == '1') {
    //             previous_PES_packet_CRC 16 bslbf
    //         }

    //         if ( PES_extension_flag == '1') {
    //             PES_private_data_flag 1 bslbf
    //             pack_header_field_flag 1 bslbf
    //             program_packet_sequence_counter_flag 1 bslbf
    //             P-STD_buffer_flag 1 bslbf
    //             reserved 3 bslbf
    //             PES_extension_flag_2 1 bslbf
    //             if ( PES_private_data_flag == '1') {
    //                 PES_private_data 128 bslbf
    //             }
    //             if (pack_header_field_flag == '1') {
    //                 pack_field_length 8 uimsbf
    //                 pack_header()
    //             }
    //             if (program_packet_sequence_counter_flag == '1') {
    //                 marker_bit 1 bslbf
    //                 program_packet_sequence_counter 7 uimsbf
    //                 marker_bit 1 bslbf
    //                 MPEG1_MPEG2_identifier 1 bslbf
    //                 original_stuff_length 6 uimsbf
    //             }

    //             if ( P-STD_buffer_flag == '1') {
    //                 '01' 2 bslbf
    //                 P-STD_buffer_scale 1 bslbf
    //                 P-STD_buffer_size 13 uimsbf
    //             }

    //             if ( PES_extension_flag_2 == '1') {
    //                 marker_bit 1 bslbf
    //                 PES_extension_field_length 7 uimsbf
    //                 stream_id_extension_flag 1 bslbf
    //                 if ( stream_id_extension_flag == '0') {
    //                     stream_id_extension 7 uimsbf
    //                 } else {
    //                     reserved 6 bslbf
    //                     tref_extension_flag 1 bslbf
    //                     if ( tref_extension_flag  '0' ) {
    //                         reserved 4 bslbf
    //                         TREF[32..30] 3 bslbf
    //                         marker_bit 1 bslbf
    //                         TREF[29..15] 15 bslbf
    //                         marker_bit 1 bslbf
    //                         TREF[14..0] 15 bslbf
    //                         marker_bit 1 bslbf
    //                     }
    //                 }

    //                 for ( i  0; i  N3; i) {
    //                     reserved 8 bslbf
    //                 }
    //             }
    //         }
    //         for (i < 0; i < N1; i++) {
    //             stuffing_byte 8 bslbf
    //         }
    //         for (i < 0; i < N2; i++) {
    //             PES_packet_data_byte 8 bslbf
    //         }
    // }
    pub fn parse_mpeg2(&mut self, bytes_reader: &mut BytesReader) -> Result<(), MpegError> {
        bytes_reader.backup();
        // log::info!("parse 0 : length: {}", bytes_reader.len());
        bytes_reader.read_bytes(3)?;
        self.stream_id = bytes_reader.read_u8()?;
        self.pes_packet_length = bytes_reader.read_u16::<BigEndian>()?;

        if self.pes_packet_length as usize > bytes_reader.len() {
            bytes_reader.restore();
            let not_enouth_bytes_err = MpegPsError {
                value: MpegPsErrorValue::NotEnoughBytes,
            };
            return Err(MpegError {
                value: crate::errors::MpegErrorValue::MpegPsError(not_enouth_bytes_err),
            });
        }

        let bytes_5 = bytes_reader.read_u8()?;
        assert!(bytes_5 >> 6 == 0b10);
        self.pes_scrambling_control = (bytes_5 >> 4) & 0x03;
        self.pes_priority = (bytes_5 >> 3) & 0x01;
        self.data_alignment_indicator = (bytes_5 >> 2) & 0x01;
        self.copyright = (bytes_5 >> 1) & 0x01;
        self.original_or_copy = bytes_5 & 0x01;
        // log::info!("parse 1");
        let bytes_6 = bytes_reader.read_u8()?;
        self.pts_dts_flags = (bytes_6 >> 6) & 0x03;
        self.escr_flag = (bytes_6 >> 5) & 0x01;
        self.es_rate_flag = (bytes_6 >> 4) & 0x01;
        self.dsm_trick_mode_flag = (bytes_6 >> 3) & 0x01;
        self.additional_copy_info_flag = (bytes_6 >> 2) & 0x01;
        self.pes_crc_flag = (bytes_6 >> 1) & 0x01;
        self.pes_extension_flag = bytes_6 & 0x01;

        self.pes_header_data_length = bytes_reader.read_u8()?;
        // log::info!("parse 2: {}", self.pes_header_data_length);
        let cur_bytes_len = bytes_reader.len();

        if self.pts_dts_flags == 0x02 {
            let next_byte = bytes_reader.read_u8()?;
            assert!(next_byte >> 4 == 0b0010);
            self.pts = (next_byte as u64 >> 1) & 0x07;
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        } else if self.pts_dts_flags == 0x03 {
            let next_byte = bytes_reader.read_u8()?;
            assert!(next_byte >> 4 == 0b0011);
            self.pts = (next_byte as u64 >> 1) & 0x07;
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

            let next_byte_1 = bytes_reader.read_u8()?;
            assert!(next_byte_1 >> 4 == 0b0011);
            self.dts = (next_byte_1 as u64 >> 1) & 0x07;
            self.dts = (self.dts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.dts = (self.dts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        }
        // log::info!("parse 3");
        if self.escr_flag == 0x01 {
            let next_byte = bytes_reader.read_u8()?;
            self.escr_base = (next_byte as u64 >> 3) & 0x07;
            self.escr_base = (self.escr_base << 2) | (next_byte as u64 & 0x03);

            let next_2_bytes = bytes_reader.read_u16::<BigEndian>()? as u64;
            self.escr_base = (self.escr_base << 13) | (next_2_bytes >> 3);
            self.escr_base = (self.escr_base << 2) | (next_2_bytes & 0x03);

            let next_2_bytes_2 = bytes_reader.read_u16::<BigEndian>()? as u64;
            self.escr_base = (self.escr_base << 13) | (next_2_bytes_2 >> 3);

            self.escr_extension = next_2_bytes as u32 & 0x03;
            self.escr_extension =
                (self.escr_extension << 7) | (bytes_reader.read_u8()? as u32 >> 1);
        }
        // log::info!("parse 4");
        if self.es_rate_flag == 0x01 {
            self.es_rate = (bytes_reader.read_u24::<BigEndian>()? >> 1) & 0x3FFFFF;
        }

        if self.dsm_trick_mode_flag == 0x01 {
            let next_byte = bytes_reader.read_u8()?;
            self.trick_mode_control = next_byte >> 5;
        }
        // log::info!("parse 5");
        if self.additional_copy_info_flag == 0x01 {
            self.additional_copy_info = bytes_reader.read_u8()? & 0x7F;
        }

        if self.pes_crc_flag == 0x01 {
            self.previous_pes_packet_crc = bytes_reader.read_u16::<BigEndian>()?;
        }

        if self.pes_extension_flag == 0x01 {}

        let left_pes_header_len =
            self.pes_header_data_length as usize - (cur_bytes_len - bytes_reader.len());
        //log::info!("parse 6: {}", left_pes_header_len);
        if left_pes_header_len > 0 {
            bytes_reader.read_bytes(left_pes_header_len)?;
        }

        let payload_len =
            self.pes_packet_length as usize - self.pes_header_data_length as usize - 3;
        // log::info!("parse 7 : {}", payload_len);
        self.payload = bytes_reader.read_bytes(payload_len)?;

        // log::info!("pes pts: {},dts: {}", self.pts / 90, self.dts / 90);

        Ok(())
    }

    pub fn parse_mpeg1(&mut self, bytes_reader: &mut BytesReader) -> Result<(), MpegError> {
        bytes_reader.read_bytes(3)?;
        self.stream_id = bytes_reader.read_u8()?;
        self.pes_packet_length = bytes_reader.read_u16::<BigEndian>()?;

        let cur_bytes_len = bytes_reader.len();

        while bytes_reader.advance_u8()? == 0xFF {
            bytes_reader.read_u8()?;
        }

        if (bytes_reader.advance_u8()? >> 6) == 0x01 {
            bytes_reader.read_u16::<BigEndian>()?;
        }

        let next_byte = bytes_reader.read_u8()?;
        let first_4_bits = next_byte >> 4;

        if first_4_bits == 0x02 {
            self.pts = (next_byte as u64 >> 1) & 0x07;
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        } else if first_4_bits == 0x03 {
            self.pts = (next_byte as u64 >> 1) & 0x07;
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.pts = (self.pts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);

            let next_byte_2 = bytes_reader.read_u8()?;
            self.dts = (next_byte_2 as u64 >> 1) & 0x07;
            self.dts = (self.dts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
            self.dts = (self.dts << 15) | (bytes_reader.read_u16::<BigEndian>()? as u64 >> 1);
        } else {
            assert_eq!(next_byte, 0x0F);
        }

        let payload_len = self.pes_packet_length as usize - (cur_bytes_len - bytes_reader.len());
        self.payload = bytes_reader.read_bytes(payload_len)?;

        Ok(())
    }
}

pub struct PesMuxer {
    pub bytes_writer: BytesWriter,
}

impl Default for PesMuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl PesMuxer {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.bytes_writer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    //http://dvdnav.mplayerhq.hu/dvdinfo/pes-hdr.html
    pub fn write_pes_header(
        &mut self,
        payload_data_length: usize,
        stream_data: &Pes,
        h264_h265_with_aud: bool,
    ) -> Result<(), MpegError> {
        /*pes start code 3 bytes*/
        self.bytes_writer.write_u8(0x00)?; //0
        self.bytes_writer.write_u8(0x00)?; //1
        self.bytes_writer.write_u8(0x01)?; //2

        /*stream id 1 byte*/
        self.bytes_writer.write_u8(stream_data.stream_id)?; //3

        /*pes packet length 2 bytes*/
        self.bytes_writer.write_u8(0x00)?; //4
        self.bytes_writer.write_u8(0x00)?; //5

        /*first flag 1 byte*/
        self.bytes_writer.write_u8(0x80)?; //6

        if stream_data.data_alignment_indicator > 0 {
            self.bytes_writer.or_u8_at(6, 0x04)?;
        }

        let mut flags: u8 = 0x00;
        let mut length: u8 = 0x00;
        if define::PTS_NO_VALUE != stream_data.pts as i64 {
            flags |= 0x80;
            length += 5;
        }

        if define::PTS_NO_VALUE != stream_data.dts as i64 && stream_data.dts != stream_data.pts {
            flags |= 0x40;
            length += 5;
        }

        /*second flag 1 byte*/
        self.bytes_writer.write_u8(flags)?; //7

        /*pes header data length*/
        self.bytes_writer.write_u8(length)?; //8

        //http://dvdnav.mplayerhq.hu/dvdinfo/pes-hdr.html
        /*The flags has 0x80 means that it has pts -- 5 bytes*/
        if (flags & 0x80) > 0 {
            let b9 = ((flags >> 2) & 0x30)/* 0011/0010 */ | (((stream_data.pts >> 30) & 0x07) << 1) as u8 /* PTS 30-32 */ | 0x01 /* marker_bit */;
            self.bytes_writer.write_u8(b9)?; //9

            let b10 = (stream_data.pts >> 22) as u8; /* PTS 22-29 */
            self.bytes_writer.write_u8(b10)?; //10

            let b11 = ((stream_data.pts >> 14) & 0xFE) as u8 /* PTS 15-21 */ | 0x01; /* marker_bit */
            self.bytes_writer.write_u8(b11)?; //11

            let b12 = (stream_data.pts >> 7) as u8; /* PTS 7-14 */
            self.bytes_writer.write_u8(b12)?; //12

            let b13 = ((stream_data.pts << 1) & 0xFE) as u8 /* PTS 0-6 */ | 0x01; /* marker_bit */
            self.bytes_writer.write_u8(b13)?; //13
        }

        /*The flags has 0x40 means that it has dts -- 5 bytes*/
        if (flags & 0x40) > 0 {
            let b14 = 0x10 /* 0001 */ | (((stream_data.dts >> 30) & 0x07) << 1) as u8 /* DTS 30-32 */ | 0x01 /* marker_bit */;
            self.bytes_writer.write_u8(b14)?;

            let b15 = (stream_data.dts >> 22) as u8; /* DTS 22-29 */
            self.bytes_writer.write_u8(b15)?;

            let b16 =  ((stream_data.dts >> 14) & 0xFE) as u8 /* DTS 15-21 */ | 0x01 /* marker_bit */;
            self.bytes_writer.write_u8(b16)?;

            let b17 = (stream_data.dts >> 7) as u8; /* DTS 7-14 */
            self.bytes_writer.write_u8(b17)?;

            let b18 = ((stream_data.dts << 1) as u8 & 0xFE) /* DTS 0-6 */ | 0x01 /* marker_bit */;
            self.bytes_writer.write_u8(b18)?;
        }

        if define::epsi_stream_type::PSI_STREAM_H264 == stream_data.codec_id && !h264_h265_with_aud
        {
            let header: [u8; 6] = [0x00, 0x00, 0x00, 0x01, 0x09, 0xF0];
            self.bytes_writer.write(&header)?;
        }

        let pes_payload_length =
            self.bytes_writer.len() - define::PES_HEADER_LEN as usize + payload_data_length;

        /*pes header -- update pes packet length*/
        if pes_payload_length > 0xFFFF {
            //only video data can exceed the 0xFFFF length,0 represet unlimited length
            self.bytes_writer.write_u8_at(4, 0x00)?;
            self.bytes_writer.write_u8_at(5, 0x00)?;
        } else {
            self.bytes_writer
                .write_u8_at(4, (pes_payload_length >> 8) as u8)?;
            self.bytes_writer
                .write_u8_at(5, (pes_payload_length) as u8)?;
        }

        Ok(())
    }
}
