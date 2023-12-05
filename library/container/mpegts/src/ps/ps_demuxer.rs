use super::{
    errors::MpegPsError,
    pack_header::{MpegType, PsPackHeader},
    psd::ProgramStreamDirectory,
    psm::ProgramStreamMap,
    system_header::PsSystemHeader,
};
use byteorder::BigEndian;
use std::collections::HashMap;
use {bytes::BytesMut, bytesio::bytes_reader::BytesReader};

use crate::{
    define::{
        epes_stream_id::{self},
        epsi_stream_type,
    },
    errors::MpegError,
    pes::Pes,
};
//(pts: u64,dts:u64, stream_type: u8, payload: BytesMut)
pub type OnFrameFn = Box<dyn Fn(u64, u64, u8, BytesMut) -> Result<(), MpegPsError> + Send + Sync>;

#[derive(Default)]
pub struct AVStream {
    pub stream_id: u8,
    stream_type: u8,
    pts: u64,
    dts: u64,
    buffer: BytesMut,
}

pub struct PsDemuxer {
    reader: BytesReader,
    pack_header: PsPackHeader,
    psm: ProgramStreamMap,
    psd: ProgramStreamDirectory,
    system_header: PsSystemHeader,
    pes: Pes,
    streams: HashMap<u8, AVStream>,
    on_frame_handler: OnFrameFn,
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

impl PsDemuxer {
    pub fn new(on_frame_handler: OnFrameFn) -> Self {
        Self {
            reader: BytesReader::new(BytesMut::default()),
            pack_header: PsPackHeader::default(),
            psm: ProgramStreamMap::default(),
            psd: ProgramStreamDirectory::default(),
            system_header: PsSystemHeader::default(),
            pes: Pes::default(),
            streams: HashMap::default(),
            on_frame_handler,
        }
    }
    pub fn demux(&mut self, data: BytesMut) -> Result<(), MpegError> {
        self.reader.extend_from_slice(&data[..]);
        //log::info!("demux: {}", self.reader.len());

        while !self.reader.is_empty() {
            let prefix_code = self.reader.advance_bytes(4)?;

            if prefix_code[0] != 0x00 || prefix_code[1] != 0x00 || prefix_code[2] != 0x01 {
                self.reader.read_u8()?;
                continue;
            }

            match prefix_code[3] {
                epes_stream_id::PES_SID_START => {
                    log::trace!(" epes_stream_id::PES_SID_START");
                    self.pack_header.parse(&mut self.reader)?;
                }
                epes_stream_id::PES_SID_SYS => {
                    log::trace!(" epes_stream_id::PES_SID_SYS");
                    self.system_header.parse(&mut self.reader)?;
                }
                epes_stream_id::PES_SID_PSM => {
                    log::trace!(" epes_stream_id::PES_SID_PSM");
                    self.psm.parse(&mut self.reader)?;
                    for stream in &self.psm.stream_map {
                        self.streams
                            .entry(stream.elementary_stream_id)
                            .or_insert(AVStream {
                                stream_id: stream.elementary_stream_id,
                                stream_type: stream.stream_type,
                                ..Default::default()
                            });
                    }
                }
                epes_stream_id::PES_SID_PSD => {
                    log::trace!(" epes_stream_id::PES_SID_PSD");
                    self.psd.parse(&mut self.reader)?;
                }
                epes_stream_id::PES_SID_PRIVATE_1
                | epes_stream_id::PES_SID_PADDING
                | epes_stream_id::PES_SID_PRIVATE_2
                | epes_stream_id::PES_SID_ECM
                | epes_stream_id::PES_SID_EMM
                | epes_stream_id::PES_SID_DSMCC
                | epes_stream_id::PES_SID_13522
                | epes_stream_id::PES_SID_H222_A
                | epes_stream_id::PES_SID_H222_B
                | epes_stream_id::PES_SID_H222_C
                | epes_stream_id::PES_SID_H222_D
                | epes_stream_id::PES_SID_H222_E
                | epes_stream_id::PES_SID_ANCILLARY
                | epes_stream_id::PES_SID_MPEG4_SL
                | epes_stream_id::PES_SID_MPEG4_FLEX => {
                    self.parse_packet()?;
                }

                epes_stream_id::PES_SID_AUDIO | epes_stream_id::PES_SID_VIDEO => {
                    // if prefix_code[3] != 224 {
                    //     log::info!("code;{}", prefix_code[3]);
                    // }
                    //log::info!("stream_id: {}", prefix_code[3]);
                    match self.pack_header.mpeg_type {
                        MpegType::Mpeg1 => {
                            // log::info!("mpeg1");
                            self.pes.parse_mpeg1(&mut self.reader)?;
                        }
                        MpegType::Mpeg2 => {
                            // log::info!("mpeg2: {:?}",self.reader.get_remaining_bytes());
                            self.pes.parse_mpeg2(&mut self.reader)?;
                        }
                        MpegType::Unknown => {
                            log::warn!("unknow mpeg type");
                        }
                    }
                    self.parse_avstream()?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn parse_packet(&mut self) -> Result<(), MpegPsError> {
        //start code + stream_id
        self.reader.read_bytes(4)?;
        let packet_length = self.reader.read_u16::<BigEndian>()?;
        self.reader.read_bytes(packet_length as usize)?;

        Ok(())
    }

    fn parse_avstream(&mut self) -> Result<(), MpegPsError> {
        if let Some(stream) = self.streams.get_mut(&self.pes.stream_id) {
            match stream.stream_type {
                epsi_stream_type::PSI_STREAM_H264 | epsi_stream_type::PSI_STREAM_H265 => {
                    stream.buffer.extend_from_slice(&self.pes.payload[..]);

                    while !stream.buffer.is_empty() {
                        /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
                        /*  |         |              |      |             */
                        /*  -----------              --------             */
                        /*   first_pos         distance_to_first_pos      */
                        if let Some(first_pos) = find_start_code(&stream.buffer[..]) {
                            let mut nalu_with_start_code = if let Some(distance_to_first_pos) =
                                find_start_code(&stream.buffer[first_pos + 3..])
                            {
                                let mut second_pos = first_pos + 3 + distance_to_first_pos;
                                //judge if the start code is [0x00,0x00,0x00,0x01]
                                if second_pos > 0 && stream.buffer[second_pos - 1] == 0 {
                                    second_pos -= 1;
                                }
                                stream.buffer.split_to(second_pos)
                            } else {
                                break;
                            };

                            let nalu = nalu_with_start_code.split_off(first_pos + 3);
                            (self.on_frame_handler)(
                                stream.pts,
                                stream.dts,
                                stream.stream_type,
                                nalu,
                            )?;
                        } else {
                            break;
                        }
                    }
                }
                epsi_stream_type::PSI_STREAM_AAC => {
                    log::info!(" receive aac");
                    if stream.dts != self.pes.dts && !stream.buffer.is_empty() {
                        (self.on_frame_handler)(
                            stream.pts,
                            stream.dts,
                            stream.stream_type,
                            self.pes.payload.clone(),
                        )?;
                        log::info!(" receive aac 2");
                        stream.buffer.clear();
                    }

                    stream.buffer.extend_from_slice(&self.pes.payload[..]);
                }
                _ => {
                    log::error!("unprocessed codec type: {}", stream.stream_type);
                }
            }
            stream.pts = self.pes.pts;
            stream.dts = self.pes.dts;
        }

        Ok(())
    }
}
