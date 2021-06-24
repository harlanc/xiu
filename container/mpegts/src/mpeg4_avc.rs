use super::errors::MpegTsParseError;
use byteorder::BigEndian;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;

pub struct Sps {
    pub size: u16,
    pub data: BytesMut,
}

pub struct Pps {
    pub size: u16,
    pub data: BytesMut,
}

pub struct Mpeg4Avc {
    profile: u8,
    compatibility: u8,
    level: u8,
    nalu: u8,

    nb_sps: u8,
    nb_pps: u8,

    sps: [Sps; 32],
    pps: [Pps; 256],

    //extension
    chroma_format_idc: u8,
    bit_depth_luma_minus8: u8,
    bit_depth_chroma_minus8: u8,

    data: [u8; 4 * 1024],
    off: i32,
}

pub struct Mpeg4AvcReader {
    pub bytes_reader: BytesReader,
    pub mpeg4_avc: Mpeg4Avc,
}

impl Mpeg4AvcReader {
    pub fn decoder_configuration_record_load(&mut self) -> Result<(), MpegTsParseError> {
        self.bytes_reader.read_u8()?;

        self.mpeg4_avc.profile = self.bytes_reader.read_u8()?;
        self.mpeg4_avc.compatibility = self.bytes_reader.read_u8()?;
        self.mpeg4_avc.level = self.bytes_reader.read_u8()?;

        self.mpeg4_avc.nalu = self.bytes_reader.read_u8()? & 0x03 + 1;
        self.mpeg4_avc.nb_sps = self.bytes_reader.read_u8()? & 0x1f;

        Ok(())
    }
}

pub struct Mpeg4AvcWriter {
    pub bytes_writer: BytesWriter,
    pub mpeg4_avc: Mpeg4Avc,
}

impl Mpeg4AvcWriter {
    pub fn decoder_configuration_record_save(&mut self) -> Result<(), MpegTsParseError> {
        self.bytes_writer.write_u8(1)?;
        self.bytes_writer.write_u8(self.mpeg4_avc.profile)?;

        self.bytes_writer.write_u8(self.mpeg4_avc.compatibility)?;
        self.bytes_writer.write_u8(self.mpeg4_avc.level)?;
        self.bytes_writer
            .write_u8((self.mpeg4_avc.nalu - 1) | 0xFC)?;

        self.bytes_writer.write_u8(self.mpeg4_avc.nb_sps | 0xE0)?;

        for i in 0..self.mpeg4_avc.nb_sps as usize {
            self.bytes_writer
                .write_u16::<BigEndian>(self.mpeg4_avc.sps[i].size)?;
            self.bytes_writer.write(&self.mpeg4_avc.sps[i].data[..])?;
        }
        Ok(())
    }
}
