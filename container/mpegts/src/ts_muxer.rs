use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::io::Write;

use super::define;
use super::define::epat_pid;
use super::define::epes_stream_id;
use super::define::ts;
use super::errors::MpegTsError;
use super::errors::MpegTsErrorValue;
use super::pat;
use super::pes;
use super::pmt;
use super::utils;
use byteorder::BigEndian;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;
use rand::Open01;
use rtmp::utils::print;
use tokio::stream;

pub struct TsMuxer {
    pub bytes_writer: BytesWriter,
    pat_continuity_counter: u8,
    pmt_continuity_counter: u8,
    h264_h265_with_aud: bool,
    pid: u16,
    pat_period: i64,
    pcr_period: i64,
    pcr_clock: i64,
    pat: pat::Pat,
    cur_pmt_index: usize,
    cur_stream_index: usize,

    packet_number: usize,
}

impl TsMuxer {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            pat_continuity_counter: 0,
            pmt_continuity_counter: 0,
            h264_h265_with_aud: false,
            pid: 0x0100,
            pat_period: 0,
            pcr_period: 80 * 90,
            pcr_clock: 0,
            pat: pat::Pat::default(),
            cur_pmt_index: 0,
            cur_stream_index: 0,
            packet_number: 0,
        }
    }

    pub fn reset(&mut self) {
        self.pat_period = 0;
        self.pcr_period = 80 * 90;
        self.pcr_clock = 0;

        self.packet_number = 0;
    }

    pub fn get_data(&mut self) -> BytesMut {
        return self.bytes_writer.extract_current_bytes();
    }

    pub fn write(
        &mut self,
        pid: u16,
        pts: i64,
        dts: i64,
        flags: u16,
        payload: BytesMut,
    ) -> Result<(), MpegTsError> {
        if (flags & define::MPEG_FLAG_H264_H265_WITH_AUD) > 0 {
            self.h264_h265_with_aud = true;
        } else {
            self.h264_h265_with_aud = false;
        }

        //print!("pes payload length {}\n", payload.len());
        //self.packet_number += payload.len();
        //print!("pes payload sum length {}\n", self.payload_sum);

        self.find_stream(pid)?;

        let cur_pmt = self.pat.pmt.get_mut(self.cur_pmt_index).unwrap();
        let cur_stream = cur_pmt.streams.get_mut(self.cur_stream_index).unwrap();

        if 0x1FFF == cur_pmt.pcr_pid
            || (define::epes_stream_id::PES_SID_VIDEO
                == (cur_stream.stream_id & define::epes_stream_id::PES_SID_VIDEO)
                && (cur_pmt.pcr_pid != cur_stream.pid))
        {
            cur_pmt.pcr_pid = cur_stream.pid;
            self.pat_period = 0;
        }

        if cur_pmt.pcr_pid == cur_stream.pid {
            self.pcr_clock += 1;
        }

        cur_stream.pts = pts;
        cur_stream.dts = dts;

        if (flags & define::MPEG_FLAG_IDR_FRAME) > 0 {
            cur_stream.data_alignment_indicator = 1; // idr frame
        } else {
            cur_stream.data_alignment_indicator = 0; // idr frame
        }

        if 0 == self.pat_period || (self.pat_period + define::PAT_PERIOD) <= dts {
            self.pat_period = dts;
            let pat_data = pat::PatMuxer::new().write(self.pat.clone())?;

            self.write_ts_header_for_pat_pmt(
                epat_pid::PAT_TID_PAS,
                pat_data,
                self.pat_continuity_counter,
            )?;
            self.pat_continuity_counter = (self.pat_continuity_counter + 1) % 16;
            self.packet_number += 1;

            for pmt_data in &mut self.pat.pmt.clone() {
                let payload_data = pmt::PmtMuxer::new().write(pmt_data)?;
                self.write_ts_header_for_pat_pmt(
                    pmt_data.pid,
                    payload_data,
                    self.pmt_continuity_counter,
                )?;
                self.pmt_continuity_counter = (self.pmt_continuity_counter + 1) % 16;
                self.packet_number += 1;
            }
        }

        self.write_pes(payload)?;

        Ok(())
    }

    pub fn write_ts_header_for_pat_pmt(
        &mut self,
        pid: u16,
        payload: BytesMut,
        continuity_counter: u8,
    ) -> Result<(), MpegTsError> {
        /*sync byte*/
        self.bytes_writer.write_u8(0x47)?; //0
                                           /*PID 13 bits*/
        self.bytes_writer
            .write_u8(0x40 | ((pid >> 8) as u8 & 0x1F))?; //1

        self.bytes_writer.write_u8(pid as u8 & 0xFF)?; //2

        self.bytes_writer
            .write_u8(0x10 | (continuity_counter & 0xFF))?;

        // match pid {
        //     epat_pid::PAT_TID_PAS => {
        //         self.bytes_writer
        //             .write_u8(0x10 | (self.pat_continuity_counter & 0xFF))?;
        //         self.pat_continuity_counter = (self.pat_continuity_counter + 1) % 16;
        //     }
        //     epat_pid::PAT_TID_PMS => {
        //         self.bytes_writer
        //             .write_u8(0x10 | (self.pmt_continuity_counter & 0xFF))?;
        //         self.pmt_continuity_counter = (self.pmt_continuity_counter + 1) % 16;
        //     }

        //     _ => {}
        // }

        /*adaption field control*/
        self.bytes_writer.write_u8(0x00)?; //4

        /*payload data*/
        self.bytes_writer.write(&payload)?;

        let left_size = ts::TS_PACKET_SIZE - payload.len() as u8 - 5;
        for _ in 0..left_size {
            self.bytes_writer.write_u8(0xFF)?;
        }
        Ok(())
    }
    //2.4.3.6 PES packet P35
    pub fn write_pes(&mut self, payload: BytesMut) -> Result<(), MpegTsError> {
        let mut is_start: bool = true;
        let mut payload_reader = BytesReader::new(payload);

        let cur_pcr_pid = self.pat.pmt.get(self.cur_pmt_index).unwrap().pcr_pid;

        while payload_reader.len() > 0 {
            //write ts header
            let mut ts_header = BytesWriter::new();
            self.write_ts_header_for_pes(&mut ts_header, is_start, cur_pcr_pid)?;
            self.packet_number += 1;

            //write pes header
            let mut pes_header = BytesWriter::new();
            if is_start {
                self.write_pes_header(&mut pes_header)?;

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

            let mut ts_header_length = ts_header.len();
            if (ts_header.get(3).unwrap() & 0x20) == 0 {
                ts_header_length -= 2;
            }
            let pes_header_length: usize = pes_header.len();
            let mut payload_length = payload_reader.len();

            let mut stuffing_length = define::TS_PACKET_SIZE as i32
                - (ts_header_length + pes_header_length + payload_length) as i32;

            if self.packet_number == 253 || self.packet_number == 254 {
                print!("packet number  is 9 {}", self.packet_number);
            }

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
                    // /*add flag*/
                    if stuffing_length == 0 {
                        ts_header.pop_bytes(1);
                    } else if stuffing_length > 1 {
                        /*remove flag*/
                        stuffing_length -= 1;
                    }
                }
                for _ in 0..stuffing_length {
                    ts_header.write_u8(0xFF)?;
                }
            } else {
                if (ts_header.get(3).unwrap() & 0x20) == 0 {
                    // let length = ts_header.len();
                    // print!(
                    //     "ts header length {} and stuffing length is {}\n",
                    //     length, stuffing_length
                    // );
                    ts_header.pop_bytes(2);
                }
                payload_length = define::TS_PACKET_SIZE - ts_header_length - pes_header_length;
            }

            is_start = false;

            let data = payload_reader.read_bytes(payload_length)?;
            print::print(data.clone());

            self.bytes_writer.append(&mut ts_header);

            //print!("==================");

            //print::print(pes_header.get_current_bytes());

            self.bytes_writer.append(&mut pes_header);

            self.bytes_writer.write(&data[..])?;
            //print!("packet number pes {}", self.packet_number);

            //self.packet_number += 1;

            if self.packet_number == 12 {
                print!("packet number  is 9 {}", self.packet_number);
            }
        }
        Ok(())
    }
    pub fn write_ts_header_for_pes(
        &mut self,
        // stream_data: &mut pes::Pes,
        ts_header: &mut BytesWriter,
        is_start: bool,
        pcr_pid: u16,
    ) -> Result<(), MpegTsError> {
        let cur_pmt = self.pat.pmt.get_mut(self.cur_pmt_index).unwrap();
        let stream_data = cur_pmt.streams.get_mut(self.cur_stream_index).unwrap();

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
        stream_data.continuity_counter = (stream_data.continuity_counter + 1) % 16;

        /*will be used for adaptation field length if have*/
        ts_header.write_u8(0x00)?; //4

        /*will be used for adaptation field flags if have*/
        ts_header.write_u8(0x00)?; //5

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
                let mut pcr_result: BytesWriter = BytesWriter::new();

                utils::pcr_write(&mut pcr_result, pcr * 300)?;

                ts_header.write(&pcr_result.extract_current_bytes()[..])?;
                /*adaption filed length -- add 6 for pcr length*/
                ts_header.add_u8_at(4, 6)?;
            }
        }

        Ok(())
    }
    //http://dvdnav.mplayerhq.hu/dvdinfo/pes-hdr.html
    pub fn write_pes_header(
        &mut self,
        // stream_data: &pes::Pes,
        pes_header: &mut BytesWriter,
    ) -> Result<(), MpegTsError> {
        let cur_pmt = self.pat.pmt.get(self.cur_pmt_index).unwrap();
        let stream_data = cur_pmt.streams.get(self.cur_stream_index).unwrap();

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

        if define::epsi_stream_type::PSI_STREAM_H264 == stream_data.codec_id
            && !self.h264_h265_with_aud
        {
            let header: [u8; 6] = [0x00, 0x00, 0x00, 0x01, 0x09, 0xF0];
            pes_header.write(&header)?;
        }

        if self.packet_number == 13 {
            let aa = 4;
        }

        print::print(pes_header.get_current_bytes());

        Ok(())
    }

    pub fn find_stream(&mut self, pid: u16) -> Result<(), MpegTsError> {
        let mut pmt_index: usize = 0;
        let mut stream_index: usize = 0;

        for pmt in self.pat.pmt.iter_mut() {
            for stream in pmt.streams.iter_mut() {
                if stream.pid == pid {
                    self.cur_pmt_index = pmt_index;
                    self.cur_stream_index = stream_index;

                    return Ok(());
                }
                stream_index += 1;
            }
            pmt_index += 1;
        }

        return Err(MpegTsError {
            value: MpegTsErrorValue::StreamNotFound,
        });
    }

    pub fn add_stream(&mut self, codecid: u8, extra_data: BytesMut) -> Result<u16, MpegTsError> {
        if 0 == self.pat.pmt.len() {
            self.add_program(1, BytesMut::new())?;
        }

        self.pmt_add_stream(0, codecid, extra_data)
    }

    pub fn pmt_add_stream(
        &mut self,
        pmt_index: usize,
        codecid: u8,
        extra_data: BytesMut,
    ) -> Result<u16, MpegTsError> {
        let pmt = &mut self.pat.pmt[pmt_index];

        if pmt.streams.len() == 4 {
            return Err(MpegTsError {
                value: MpegTsErrorValue::StreamCountExeceed,
            });
        }

        let mut cur_stream = pes::Pes::default(); //&mut pmt.streams[pmt.stream_count];

        cur_stream.codec_id = codecid;
        cur_stream.pid = self.pid;
        self.pid += 1;

        if utils::is_steam_type_video(codecid) {
            cur_stream.stream_id = epes_stream_id::PES_SID_VIDEO;
        } else if utils::is_steam_type_audio(codecid) {
            cur_stream.stream_id = epes_stream_id::PES_SID_AUDIO;
        } else {
            cur_stream.stream_id = epes_stream_id::PES_SID_PRIVATE_1;
        }

        if extra_data.len() > 0 {
            cur_stream.esinfo.put(extra_data);
        }

        pmt.streams.push(cur_stream);
        pmt.version_number = (pmt.version_number + 1) % 32;

        self.reset();

        Ok(self.pid - 1)
    }

    pub fn add_program(&mut self, program_number: u16, info: BytesMut) -> Result<(), MpegTsError> {
        for cur_pmt in self.pat.pmt.iter() {
            if cur_pmt.program_number == program_number {
                return Err(MpegTsError {
                    value: MpegTsErrorValue::ProgramNumberExists,
                });
            }
        }

        if self.pat.pmt.len() == 4 {
            return Err(MpegTsError {
                value: MpegTsErrorValue::PmtCountExeceed,
            });
        }
        let mut cur_pmt = pmt::Pmt::default(); //&mut self.pat.pmt[self.pat.pmt_count];

        cur_pmt.pid = self.pid;
        self.pid += 1;
        cur_pmt.program_number = program_number;
        cur_pmt.version_number = 0x00;
        cur_pmt.continuity_counter = 0;
        cur_pmt.pcr_pid = 0x1FFF;

        if info.len() > 0 {
            cur_pmt.program_info.put(&info[..]);
        }

        self.pat.pmt.push(cur_pmt);

        //self.pat.pmt_count += 1;

        Ok(())
    }
}
