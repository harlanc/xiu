use std::io::Write;

use super::define;
use super::define::epat_pid;
use super::define::ts;
use super::errors::MpegTsError;
use super::pat;
use super::pes;
use super::pmt;
use super::utils;
use byteorder::BigEndian;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;
use rand::Open01;

pub struct TsWriter {
    bytes_writer: BytesWriter,
    pat_continuity_counter: u8,
    pmt_continuity_counter: u8,
    h264_h265_with_aud: bool,
}

impl TsWriter {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            pat_continuity_counter: 0,
            pmt_continuity_counter: 0,
            h264_h265_with_aud: false,
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
    //2.4.3.6 PES packet P35
    pub fn write_pes(
        &mut self,
        pmt_data: pmt::Pmt,
        stream_data: &mut pes::Pes,
        payload: BytesMut,
    ) -> Result<(), MpegTsError> {
        let mut is_start: bool = true;
        let mut payload_reader = BytesReader::new(payload);

        let mut writer = BytesWriter::new();

        while payload_reader.len() > 0 {
            //write ts header
            let mut ts_header = BytesWriter::new();
            self.write_ts_header(stream_data, &mut ts_header, is_start, pmt_data.pcr_pid)?;

            //write pes header
            let mut pes_header = BytesWriter::new();
            if is_start {
                self.write_pes_header(stream_data, &mut pes_header)?;

                let pes_payload_length =
                    pes_header.len() - define::PES_HEADER_LEN as usize + payload_reader.len();

                /*pes header -- update pes packet length*/
                if pes_payload_length > 0xFFFF {
                    //only video data can exceed the 0xFFFF length,0 represet unlimited length
                    pes_header.write_u8_at(4, 0x00)?;
                    pes_header.write_u8_at(5, 0x00)?;
                } else {
                    pes_header.write_u8_at(4, (pes_payload_length >> 8) as u8 & 0xFF)?;
                    pes_header.write_u8_at(5, (pes_payload_length) as u8 & 0xFF)?;
                }
            }

            /*
            +-------------------------------------------------------------------------+
            |        ts header                              | PES data                |
            +-------------------------------------------------------------------------+
            | 4bytes fixed header | adaption field(stuffing)| pes header | pes payload|
            +-------------------------------------------------------------------------+
            */
            // If payload data cannot fill up the 188 bytes packet,
            // then stuffling bytes need to be filled in the adaptation field,

            let ts_header_length: usize = ts_header.len();
            let pes_header_length: usize = pes_header.len();
            let mut stuffing_length = define::TS_PACKET_SIZE
                - (ts_header_length + pes_header_length + payload_reader.len());

            let payload_length;

            if stuffing_length > 0 {
                if (ts_header.get(3).unwrap() & 0x20) > 0 {
                    /*adaption filed length -- add 6 for pcr length*/
                    ts_header.add_u8_at(4, stuffing_length as u8)?;
                } else {
                    /*adaption field control*/
                    ts_header.or_u8_at(3, 0x20)?;
                    /*AF length*/
                    stuffing_length -= 1;
                    /*adaption filed length -- set value to 1 for flags*/
                    ts_header.write_u8_at(4, stuffing_length as u8)?;
                    /*add flag*/
                    if stuffing_length > 0 {
                        ts_header.write_u8(0)?;
                    }
                    if stuffing_length > 1 {
                        /*remove flag*/
                        stuffing_length -= 1;
                    }
                }
                for _ in 0..stuffing_length {
                    ts_header.write_u8(0xFF)?;
                }
                payload_length = payload_reader.len();
            } else {
                payload_length = define::TS_PACKET_SIZE - ts_header_length - pes_header_length;
            }

            is_start = false;

            let data = payload_reader.read_bytes(payload_length)?;

            writer.append(&mut ts_header);
            writer.append(&mut pes_header);
            writer.write(&data[..])?;
        }
        Ok(())
    }
    pub fn write_ts_header(
        &mut self,
        stream_data: &mut pes::Pes,
        ts_header: &mut BytesWriter,
        is_start: bool,
        pcr_pid: u16,
    ) -> Result<(), MpegTsError> {
        /****************************************************************/
        /*        ts header 4 bytes without adaptation filed            */
        /*****************************************************************
         0                   1                   2                   3
         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
        |   sync byte   | | | |          PID            |   |   |       |
        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
        */

        /*sync byte*/
        ts_header.write_u8(0x47)?; //0

        /*PID 13 bits*/
        ts_header.write_u8(0x00 | ((stream_data.pid >> 8) as u8 & 0x1F))?; //1
        ts_header.write_u8((stream_data.pid & 0xFF) as u8)?; //2

        /*continuity counter 4 bits*/
        ts_header.write_u8(0x10 | (stream_data.continuity_counter & 0x0F) as u8)?; //3

        /*will be used for adaptation field length if have*/
        ts_header.write_u8(0x00)?; //4

        /*will be used for adaptation field flags if have*/
        ts_header.write_u8(0x00)?; //5

        stream_data.continuity_counter = (stream_data.continuity_counter + 1) % 16;

        if is_start {
            /*payload unit start indicator*/
            ts_header.or_u8_at(1, define::TS_PAYLOAD_UNIT_START_INDICATOR)?;

            if stream_data.pid == pcr_pid {
                /*adaption field control*/
                ts_header.or_u8_at(3, 0x20)?;
                /*adaption field flags*/
                ts_header.or_u8_at(5, define::AF_FLAG_PCR)?;
            }

            if (stream_data.data_alignment_indicator > 0) && define::PTS_NO_VALUE != stream_data.pts
            {
                /*adaption field control*/
                ts_header.or_u8_at(3, 0x20)?;
                /*adaption field flags*/
                ts_header.or_u8_at(5, define::AF_FLAG_RANDOM_ACCESS_INDICATOR)?;
            }
        }

        /*if has adaption field */
        if (ts_header.get(3).unwrap() & 0x20) > 0 {
            /*adaption filed length -- set value to 1 for flags*/
            ts_header.write_u8_at(4, 1)?;

            if (ts_header.get(5).unwrap() & define::AF_FLAG_PCR) > 0 {
                let pcr: i64;
                if define::PTS_NO_VALUE == stream_data.dts {
                    pcr = stream_data.pts;
                } else {
                    pcr = stream_data.dts;
                }
                let mut pcr_result: Vec<u8> = Vec::new();
                utils::pcr_write(&mut pcr_result, pcr * 300);
                ts_header.write(&pcr_result[..])?;
                /*adaption filed length -- add 6 for pcr length*/
                ts_header.add_u8_at(4, 6)?;
            }
        }

        Ok(())
    }
    //http://dvdnav.mplayerhq.hu/dvdinfo/pes-hdr.html
    pub fn write_pes_header(
        &mut self,
        stream_data: &mut pes::Pes,
        pes_header: &mut BytesWriter,
    ) -> Result<(), MpegTsError> {
        /*pes start code 3 bytes*/
        pes_header.write_u8(0x00)?; //0
        pes_header.write_u8(0x00)?; //1
        pes_header.write_u8(0x01)?; //2

        /*stream id 1 byte*/
        pes_header.write_u8(stream_data.stream_id)?; //3

        /*pes packet length 2 bytes*/
        pes_header.write_u8(0x00)?; //4
        pes_header.write_u8(0x00)?; //5

        /*first flag 1 byte*/
        pes_header.write_u8(0x80)?; //6

        if stream_data.data_alignment_indicator > 0 {
            pes_header.or_u8_at(6, 0x04)?;
        }

        let mut flags: u8 = 0x00;
        let mut length: u8 = 0x00;
        if define::PTS_NO_VALUE != stream_data.pts {
            flags |= 0x80;
            length += 5;
        }

        if define::PTS_NO_VALUE != stream_data.dts && stream_data.dts != stream_data.pts {
            flags |= 0x40;
            length += 5;
        }

        /*second flag 1 byte*/
        pes_header.write_u8(flags)?; //7

        /*pes header data length*/
        pes_header.write_u8(length)?; //8

        //http://dvdnav.mplayerhq.hu/dvdinfo/pes-hdr.html
        /*The flags has 0x80 means that it has pts -- 5 bytes*/
        if (flags & 0x80) > 0 {
            let b9 = ((flags >> 2) & 0x30)/* 0011/0010 */ | (((stream_data.pts >> 30) & 0x07) << 1) as u8 /* PTS 30-32 */ | 0x01 /* marker_bit */;
            pes_header.write_u8(b9)?; //9

            let b10 = (stream_data.pts >> 22) as u8 & 0xFF; /* PTS 22-29 */
            pes_header.write_u8(b10)?; //10

            let b11 = ((stream_data.pts >> 14) & 0xFE) as u8 /* PTS 15-21 */ | 0x01; /* marker_bit */
            pes_header.write_u8(b11)?; //11

            let b12 = (stream_data.pts >> 7) as u8 & 0xFF; /* PTS 7-14 */
            pes_header.write_u8(b12)?; //12

            let b13 = ((stream_data.pts << 1) & 0xFE) as u8 /* PTS 0-6 */ | 0x01; /* marker_bit */
            pes_header.write_u8(b13)?; //13
        }

        /*The flags has 0x40 means that it has dts -- 5 bytes*/
        if (flags & 0x40) > 0 {
            let b14 = 0x10 /* 0001 */ | (((stream_data.dts >> 30) & 0x07) << 1) as u8 /* DTS 30-32 */ | 0x01 /* marker_bit */;
            pes_header.write_u8(b14)?;

            let b15 = (stream_data.dts >> 22) as u8 & 0xFF; /* DTS 22-29 */
            pes_header.write_u8(b15)?;

            let b16 =  ((stream_data.dts >> 14) & 0xFE) as u8 /* DTS 15-21 */ | 0x01 /* marker_bit */;
            pes_header.write_u8(b16)?;

            let b17 = (stream_data.dts >> 7) as u8 & 0xFF; /* DTS 7-14 */
            pes_header.write_u8(b17)?;

            let b18 = ((stream_data.dts << 1) as u8 & 0xFE) /* DTS 0-6 */ | 0x01 /* marker_bit */;
            pes_header.write_u8(b18)?;
        }

        if define::PSI_STREAM_H264 == stream_data.codec_id && !self.h264_h265_with_aud {
            let header: [u8; 6] = [0x00, 0x00, 0x00, 0x01, 0x09, 0xF0];
            pes_header.write(&header)?;
        }

        Ok(())
    }

    pub fn find_stream(&mut self, pat: pat::Pat, pid: u16) -> Option<pes::Pes> {
        for p in &pat.pmt {
            for s in &p.streams {
                if s.pid == pid {
                    return Some(s.clone());
                }
            }
        }

        None
    }
}
