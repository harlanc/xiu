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
        stream_data: pes::Pes,
        payload: BytesMut,
    ) -> Result<(), MpegTsError> {
        let mut is_start: bool = true;
        let mut cur_index: usize = 0;
        let mut stream_data_clone = stream_data.clone();

        let mut bytes_data: Vec<u8> = Vec::new();

        while payload.len() > 0 {
            bytes_data[0] = 0x47;
            bytes_data[1] = 0x00 | ((stream_data_clone.pid >> 8) as u8 & 0x1F);
            bytes_data[2] = (stream_data_clone.pid & 0xFF) as u8;
            bytes_data[3] = 0x10 | (stream_data_clone.continuity_counter & 0x0F) as u8;
            bytes_data[4] = 0x00;
            bytes_data[5] = 0x00;

            stream_data_clone.continuity_counter = (stream_data_clone.continuity_counter + 1) % 16;

            if is_start && (stream_data_clone.pid == pmt_data.pcr_pid) {
                bytes_data[3] |= 0x20;
                bytes_data[5] |= define::AF_FLAG_PCR;
            }

            if is_start
                && (stream_data_clone.data_alignment_indicator > 0)
                && define::PTS_NO_VALUE != stream_data_clone.pts
            {
                bytes_data[3] |= 0x20;
                bytes_data[5] |= define::AF_FLAG_RANDOM_ACCESS_INDICATOR;
            }

            if (bytes_data[3] & 0x20) > 0 {
                bytes_data[4] = 1;

                if (bytes_data[5] & define::AF_FLAG_PCR) > 0 {
                    let mut pcr: i64 = 0;
                    if define::PTS_NO_VALUE == stream_data_clone.dts {
                        pcr = stream_data_clone.pts;
                    } else {
                        pcr = stream_data_clone.dts;
                    }

                    let mut pcr_result: Vec<u8> = Vec::new();
                    utils::pcr_write(&mut pcr_result, pcr * 300);

                    bytes_data.append(&mut pcr_result);
                    bytes_data[4] += 6;
                }
                cur_index = (define::TS_HEADER_LEN + 1 + bytes_data[4]) as usize;
            } else {
                cur_index = define::TS_HEADER_LEN as usize;
            }

            let mut save_cur_index = cur_index;

            if is_start {
                bytes_data[1] |= define::TS_PAYLOAD_UNIT_START_INDICATOR;

                let mut pes_header: Vec<u8> = Vec::new();
                self.write_pes_header(stream_data_clone.clone(), &mut pes_header);
                bytes_data.append(&mut pes_header);

                cur_index += pes_header.len();

                if define::PSI_STREAM_H264 == stream_data.codec_id && !self.h264_h265_with_aud {
                    bytes_data[cur_index] = 0x00;
                    cur_index += 1;
                    bytes_data[cur_index] = 0x00;
                    cur_index += 1;
                    bytes_data[cur_index] = 0x00;
                    cur_index += 1;
                    bytes_data[cur_index] = 0x01;
                    cur_index += 1;
                    bytes_data[cur_index] = 0x09;
                    cur_index += 1;
                    bytes_data[cur_index] = 0xF0;
                    cur_index += 1;
                }

                let pes_length =
                    cur_index - save_cur_index - define::PES_HEADER_LEN as usize + payload.len();

                if pes_length > 0xFFFF {
                    bytes_data[save_cur_index + 4] = 0x00;
                    bytes_data[save_cur_index + 5] = 0x00;
                } else {
                    bytes_data[save_cur_index + 4] = (pes_length >> 8) as u8 & 0xFF;
                    bytes_data[save_cur_index + 5] = (pes_length) as u8 & 0xFF;
                }
            }


        }
        Ok(())
    }

    pub fn write_pes_header(&mut self, stream_data: pes::Pes, pes_header: &mut Vec<u8>) {
        let mut flags: u8 = 0x00;
        let mut length: u8 = 0x00;

        pes_header[0] = 0x00;
        pes_header[1] = 0x00;
        pes_header[2] = 0x01;
        pes_header[3] = stream_data.stream_id;

        pes_header[6] = 0x80;

        if stream_data.data_alignment_indicator > 0 {
            pes_header[6] |= 0x04;
        }

        if define::PTS_NO_VALUE != stream_data.pts {
            flags |= 0x80;
            length += 5;
        }

        if define::PTS_NO_VALUE != stream_data.dts && stream_data.dts != stream_data.pts {
            flags |= 0x40;
            length += 5;
        }

        pes_header[7] = flags;
        pes_header[8] = length;

        if (flags & 0x80) > 0 {
            pes_header[9] = ((flags >> 2) & 0x30)/* 0011/0010 */ | (((stream_data.pts >> 30) & 0x07) << 1) as u8 /* PTS 30-32 */ | 0x01 /* marker_bit */;
            pes_header[10] = (stream_data.pts >> 22) as u8 & 0xFF; /* PTS 22-29 */
            pes_header[11] = ((stream_data.pts >> 14) & 0xFE) as u8 /* PTS 15-21 */ | 0x01 /* marker_bit */;
            pes_header[12] = (stream_data.pts >> 7) as u8 & 0xFF; /* PTS 7-14 */
            pes_header[13]  = ((stream_data.pts << 1) & 0xFE) as u8 /* PTS 0-6 */ | 0x01 /* marker_bit */;
        }

        if (flags & 0x40) > 0 {
            pes_header[13] = 0x10 /* 0001 */ | (((stream_data.dts >> 30) & 0x07) << 1) as u8 /* DTS 30-32 */ | 0x01 /* marker_bit */;
            pes_header[14] = (stream_data.dts >> 22) as u8 & 0xFF; /* DTS 22-29 */
            pes_header[15] = ((stream_data.dts >> 14) & 0xFE) as u8 /* DTS 15-21 */ | 0x01 /* marker_bit */;
            pes_header[16] = (stream_data.dts >> 7) as u8 & 0xFF; /* DTS 7-14 */
            pes_header[17]= ((stream_data.dts << 1) as u8 & 0xFE) /* DTS 0-6 */ | 0x01 /* marker_bit */;
        }
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
