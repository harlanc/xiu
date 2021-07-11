use super::define::h264_nal_type;
use super::errors::MpegAvcError;
use byteorder::BigEndian;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;
use std::vec::Vec;

const H264_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

pub struct Sps {
    pub size: u16,
    pub data: BytesMut,
}

impl Sps {
    pub fn default() -> Self {
        Self {
            size: 0,
            data: BytesMut::new(),
        }
    }
}
pub struct Pps {
    pub size: u16,
    pub data: BytesMut,
}

impl Pps {
    pub fn default() -> Self {
        Self {
            size: 0,
            data: BytesMut::new(),
        }
    }
}

pub struct Mpeg4Avc {
    profile: u8,
    compatibility: u8,
    level: u8,
    nalu: u8,

    nb_sps: u8,
    nb_pps: u8,

    sps: Vec<Sps>,
    pps: Vec<Pps>,

    sps_data: BytesWriter, // pice together all the sps data
    pps_data: BytesWriter, // pice together all the pps data

    //extension
    chroma_format_idc: u8,
    bit_depth_luma_minus8: u8,
    bit_depth_chroma_minus8: u8,

    data: Vec<u8>, //[u8; 4 * 1024],
    off: i32,
}

impl Mpeg4Avc {
    pub fn default() -> Self {
        Self {
            profile: 0,
            compatibility: 0,
            level: 0,
            nalu: 0,
            nb_pps: 0,
            nb_sps: 0,

            sps: Vec::new(),
            pps: Vec::new(),

            sps_data: BytesWriter::new(),
            pps_data: BytesWriter::new(),

            chroma_format_idc: 0,
            bit_depth_chroma_minus8: 0,
            bit_depth_luma_minus8: 0,

            data: Vec::new(),
            off: 0,
        }
    }
}

pub struct Mpeg4AvcProcessor {
    pub bytes_reader: BytesReader,
    pub bytes_writer: BytesWriter,
    pub mpeg4_avc: Mpeg4Avc,
    pub sps_pps_flag: bool,
}

impl Mpeg4AvcProcessor {
    pub fn new() -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bytes_writer: BytesWriter::new(),
            mpeg4_avc: Mpeg4Avc::default(),
            sps_pps_flag: false,
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bytes_reader.extend_from_slice(&data[..]);
    }

    pub fn decoder_configuration_record_load(&mut self) -> Result<(), MpegAvcError> {
        self.bytes_reader.read_u8()?;

        self.mpeg4_avc.profile = self.bytes_reader.read_u8()?;
        self.mpeg4_avc.compatibility = self.bytes_reader.read_u8()?;
        self.mpeg4_avc.level = self.bytes_reader.read_u8()?;
        self.mpeg4_avc.nalu = self.bytes_reader.read_u8()? & 0x03 + 1;

        //sps
        self.mpeg4_avc.nb_sps = self.bytes_reader.read_u8()? & 0x1F;

        for i in 0..self.mpeg4_avc.nb_sps as usize {
            self.mpeg4_avc.sps[i].size = self.bytes_reader.read_u16::<BigEndian>()?;
            self.mpeg4_avc.sps[i].data = self
                .bytes_reader
                .read_bytes(self.mpeg4_avc.sps[i].size as usize)?;

            self.mpeg4_avc.sps_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .sps_data
                .write(&self.mpeg4_avc.sps[i].data[..])?;
        }

        //pps
        self.mpeg4_avc.nb_pps = self.bytes_reader.read_u8()?;

        for i in 0..self.mpeg4_avc.nb_sps as usize {
            self.mpeg4_avc.pps[i].size = self.bytes_reader.read_u16::<BigEndian>()?;
            self.mpeg4_avc.pps[i].data = self
                .bytes_reader
                .read_bytes(self.mpeg4_avc.pps[i].size as usize)?;

            self.mpeg4_avc.pps_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .pps_data
                .write(&self.mpeg4_avc.pps[i].data[..])?;
        }

        Ok(())
    }
    //https://stackoverflow.com/questions/28678615/efficiently-insert-or-replace-multiple-elements-in-the-middle-or-at-the-beginnin
    pub fn h264_mp4toannexb(&mut self) -> Result<(), MpegAvcError> {
        while self.bytes_reader.len() > 0 {
            let size = self.get_nalu_size()?;
            let nalu_type = self.bytes_reader.read_u8()? & 0x1f;

            match nalu_type {
                h264_nal_type::H264_NAL_PPS | h264_nal_type::H264_NAL_SPS => {
                    self.sps_pps_flag = true;
                }

                h264_nal_type::H264_NAL_IDR => {
                    if !self.sps_pps_flag {
                        self.sps_pps_flag = true;

                        self.bytes_writer
                            .prepend(&self.mpeg4_avc.sps_data.extract_current_bytes()[..])?;
                        self.bytes_writer
                            .prepend(&self.mpeg4_avc.pps_data.extract_current_bytes()[..])?;
                    }
                }

                _ => {}
            }

            self.bytes_writer.write(&H264_START_CODE)?;
            let data = self.bytes_reader.read_bytes(size as usize)?;
            self.bytes_writer.write(&data[..])?;
        }

        Ok(())
    }

    pub fn get_nalu_size(&mut self) -> Result<u8, MpegAvcError> {
        let mut size: u8 = 0;

        for _ in 0..self.mpeg4_avc.nalu {
            size = self.bytes_reader.read_u8()? + size << 8;
        }
        Ok(size)
    }
}

pub struct Mpeg4AvcWriter {
    pub bytes_writer: BytesWriter,
    pub mpeg4_avc: Mpeg4Avc,
}

impl Mpeg4AvcWriter {
    pub fn decoder_configuration_record_save(&mut self) -> Result<(), MpegAvcError> {
        self.bytes_writer.write_u8(1)?;
        self.bytes_writer.write_u8(self.mpeg4_avc.profile)?;

        self.bytes_writer.write_u8(self.mpeg4_avc.compatibility)?;
        self.bytes_writer.write_u8(self.mpeg4_avc.level)?;
        self.bytes_writer
            .write_u8((self.mpeg4_avc.nalu - 1) | 0xFC)?;

        //sps
        self.bytes_writer.write_u8(self.mpeg4_avc.nb_sps | 0xE0)?;
        for i in 0..self.mpeg4_avc.nb_sps as usize {
            self.bytes_writer
                .write_u16::<BigEndian>(self.mpeg4_avc.sps[i].size)?;
            self.bytes_writer.write(&self.mpeg4_avc.sps[i].data[..])?;
        }

        //pps
        self.bytes_writer.write_u8(self.mpeg4_avc.nb_pps)?;
        for i in 0..self.mpeg4_avc.nb_pps as usize {
            self.bytes_writer
                .write_u16::<BigEndian>(self.mpeg4_avc.pps[i].size)?;
            self.bytes_writer.write(&self.mpeg4_avc.pps[i].data[..])?
        }

        match self.mpeg4_avc.profile {
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 => {
                self.bytes_writer
                    .write_u8(0xFC | self.mpeg4_avc.chroma_format_idc)?;
                self.bytes_writer
                    .write_u8(0xF8 | self.mpeg4_avc.bit_depth_luma_minus8)?;
                self.bytes_writer
                    .write_u8(0xF8 | self.mpeg4_avc.bit_depth_chroma_minus8)?;
                self.bytes_writer.write_u8(0)?;
            }
            _ => {}
        }

        Ok(())
    }
}
