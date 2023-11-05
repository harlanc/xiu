use std::collections::HashMap;

use byteorder::BigEndian;

use super::{
    errors::MpegPsError,
    pack_header::{MpegType, PsPackHeader},
    psd::ProgramStreamDirectory,
    psm::ProgramStreamMap,
    system_header::PsSystemHeader,
};
use {
    bytes::{BufMut, BytesMut},
    bytesio::{bits_reader::BitsReader, bytes_reader::BytesReader, bytes_writer::BytesWriter},
};

use crate::{
    define::{
        epes_stream_id::{self, PES_SID_SYS},
        epsi_stream_type,
    },
    errors::MpegError,
    pes::Pes,
};
//(pts: u64,dts:u64, stream_type: u8, payload: BytesMut)
pub type OnFrameFn = Box<dyn Fn(u64, u64, u8, BytesMut) -> Result<(), MpegPsError> + Send + Sync>;

#[derive(Default)]
struct AVStream {
    stream_id: u8,
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

        while data.len() > 0 {
            let prefix_code = self.reader.advance_bytes(4)?;

            if prefix_code[0] != 0x00 || prefix_code[1] != 0x00 || prefix_code[2] != 0x01 {
                self.reader.read_u8()?;
                continue;
            }

            match prefix_code[3] {
                epes_stream_id::PES_SID_START => {
                    self.pack_header.parse(&mut self.reader)?;
                }
                epes_stream_id::PES_SID_SYS => {
                    self.system_header.parse(&mut self.reader)?;
                }
                epes_stream_id::PES_SID_PSM => {
                    self.psm.parse(&mut self.reader)?;
                    for stream in &self.psm.stream_map {
                        if !self.streams.contains_key(&stream.elementary_stream_id) {
                            self.streams.insert(
                                stream.elementary_stream_id,
                                AVStream {
                                    stream_id: stream.elementary_stream_id,
                                    stream_type: stream.stream_type,
                                    ..Default::default()
                                },
                            );
                        }
                    }
                }
                epes_stream_id::PES_SID_PSD => {
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
                    match self.pack_header.mpeg_type {
                        MpegType::Mpeg1 => {
                            self.pes.parse_mpeg1(&mut self.reader)?;
                        }
                        MpegType::Mpeg2 => {
                            self.pes.parse(&mut self.reader)?;
                        }
                        MpegType::Unknown => {
                            log::error!("unknow mpeg type");
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
                    if stream.dts != self.pes.dts && stream.buffer.len() > 0 {
                        (self.on_frame_handler)(
                            stream.pts,
                            stream.dts,
                            stream.stream_type,
                            self.pes.payload.clone(),
                        )?;
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
