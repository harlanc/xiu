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
    define::epes_stream_id::{self, PES_SID_SYS},
    errors::MpegError,
    pes::Pes,
};

pub struct PsDemuxer {
    reader: BytesReader,
    pack_header: PsPackHeader,
    psm: ProgramStreamMap,
    psd: ProgramStreamDirectory,
    system_header: PsSystemHeader,
    pes: Pes,
}

impl PsDemuxer {
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
}
