use super::define::h264_nal_type;
use super::errors::MpegAacError;
use byteorder::BigEndian;
use bytes::BytesMut;
use networkio::bytes_reader::BytesReader;
use networkio::bytes_writer::BytesWriter;
use std::vec::Vec;

const AAC_FREQUENCE_SIZE: usize = 13;
const AAC_FREQUENCE: [u32; AAC_FREQUENCE_SIZE] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
];
pub struct Mpeg4Aac {
    pub profile: u8,
    pub sampling_frequency_index: u8,
    pub channel_configuration: u8,

    pub sampling_frequency: u32,
    pub channels: u8,
    pub sbr: usize,
    pub ps: usize,
    pub pce: Vec<u8>,
    pub npce: usize,
}

impl Mpeg4Aac {
    pub fn default() -> Self {
        Self {
            profile: 0,
            sampling_frequency_index: 0,
            channel_configuration: 0,
            sampling_frequency: 0,
            channels: 0,
            sbr: 0,
            ps: 0,

            pce: Vec::new(),
            npce: 0,
        }
    }
}

pub struct Mpeg4AacProcessor {
    pub bytes_reader: BytesReader,
    pub bytes_writer: BytesWriter,
    pub mpeg4_aac: Mpeg4Aac,
}
//https://blog.csdn.net/coloriy/article/details/90511746
impl Mpeg4AacProcessor {
    pub fn new() -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bytes_writer: BytesWriter::new(),
            mpeg4_aac: Mpeg4Aac::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bytes_reader.extend_from_slice(&data[..]);
    }

    pub fn audio_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        let byte_0 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.profile = (byte_0 >> 3) & 0x1F;

        let byte_1 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.sampling_frequency_index = ((byte_0 & 0x07) << 1) | ((byte_1 >> 7) & 0x01);
        self.mpeg4_aac.channel_configuration = (byte_1 >> 3) & 0x0F;
        self.mpeg4_aac.channels = self.mpeg4_aac.channel_configuration;
        self.mpeg4_aac.sampling_frequency =
            AAC_FREQUENCE[self.mpeg4_aac.sampling_frequency_index as usize];

        Ok(())
    }

    pub fn adts_save(&mut self) -> Result<(), MpegAacError> {
        let id = 0; // 0-MPEG4/1-MPEG2
        let len = (self.bytes_reader.len() + 7) as u32;
        self.bytes_writer.write_u8(0xFF)?; //0
        self.bytes_writer.write_u8(0xF0 /* 12-syncword */ | (id << 3)/*1-ID*/ | (0x00 << 2) /*2-layer*/ | 0x01 /*1-protection_absent*/)?; //1

        let profile = self.mpeg4_aac.profile;
        let sampling_frequency_index = self.mpeg4_aac.sampling_frequency_index;
        let channel_configuration = self.mpeg4_aac.channel_configuration;
        self.bytes_writer.write_u8(
            ((profile - 1) << 6)
                | ((sampling_frequency_index & 0x0F) << 2)
                | ((channel_configuration >> 2) & 0x01),
        )?; //2

        self.bytes_writer
            .write_u8(((channel_configuration & 0x03) << 6) | ((len >> 11) as u8 & 0x03))?; //3
        self.bytes_writer.write_u8((len >> 3) as u8)?; //4
        self.bytes_writer
            .write_u8((((len & 0x07) as u8) << 5) | 0x1F)?; //5
        self.bytes_writer.write_u8(0xFC)?; //6

        self.bytes_writer
            .write(&self.bytes_reader.get_remaining_bytes()[..])?;

        Ok(())
    }
}
