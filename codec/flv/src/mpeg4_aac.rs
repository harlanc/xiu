use super::bitvec::Mpeg4BitVec;
use super::define::h264_nal_type;
use super::errors::MpegAacError;
use super::errors::MpegAacErrorValue;
use bitvec::array::BitArray;
use bitvec::prelude::*;
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
    pub pce: BytesMut,
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

            pce: BytesMut::new(),
            npce: 0,
        }
    }
}

pub struct Mpeg4Bits {
    data: BytesMut,
    size: usize,
    bits: usize,
    error: u32,
}

impl Mpeg4Bits {
    pub fn default() -> Self {
        Self {
            data: BytesMut::new(),
            size: 0,
            bits: 0,
            error: 0,
        }
    }
}

pub struct Mpeg4AacProcessor {
    pub bytes_reader: BytesReader,
    pub bytes_writer: BytesWriter,
    pub bits_data: Mpeg4BitVec,
    pub mpeg4_aac: Mpeg4Aac,
}
//https://blog.csdn.net/coloriy/article/details/90511746
impl Mpeg4AacProcessor {
    pub fn new() -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bytes_writer: BytesWriter::new(),
            bits_data: Mpeg4BitVec::new(),
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

        if 0 == self.mpeg4_aac.channel_configuration {
            print!("bytes**************************=========\n");
        }

        let length = self.bytes_reader.len();
        if self.bytes_reader.len() > 0 {
            print!("bytes elft**************************=========\n");
        }

        Ok(())
    }

    pub fn audio_specific_config_load2(&mut self) -> Result<(), MpegAacError> {
        let remain_bytes = self.bytes_reader.get_remaining_bytes();
        self.bits_data.extend_from_bytesmut(remain_bytes);

        self.mpeg4_aac.profile = self.get_audio_object_type()?;
        self.mpeg4_aac.sampling_frequency_index = self.get_sampling_frequency()?;
        self.mpeg4_aac.channel_configuration = self.bits_data.read_n_bits(4)? as u8;

        let mut extension_audio_object_type: u8 = 0;
        let mut extension_sampling_frequency_index: u8 = 0;
        let mut extension_channel_configuration: u8 = 0;

        if self.mpeg4_aac.profile == 5 || self.mpeg4_aac.profile == 29 {
            extension_audio_object_type = 5;
            self.mpeg4_aac.sbr = 1;
            {
                if self.mpeg4_aac.profile == 29 {
                    self.mpeg4_aac.ps = 1;
                }
                extension_sampling_frequency_index = self.get_sampling_frequency()?;
                self.mpeg4_aac.profile = self.get_audio_object_type()?;

                if self.mpeg4_aac.profile == 22 {
                    extension_channel_configuration = self.bits_data.read_n_bits(4)? as u8;
                }
            }
        } else {
            extension_audio_object_type = 0;
        }

        match self.mpeg4_aac.profile {
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 17 | 19 | 20 | 21 | 22 | 23 => {}
            _ => {}
        }

        Ok(())
    }

    pub fn ga_specific_config_load(&mut self) -> Result<u8, MpegAacError> {
        self.bits_data.read_n_bits(1)?;

        if self.bits_data.read_n_bits(1)? > 0 {
            self.bits_data.read_n_bits(14)?;
        }

        if 0 == self.mpeg4_aac.channel_configuration {
            let mut cur_bits_data = Mpeg4BitVec::new();
            cur_bits_data.extend_from_bytesmut(self.mpeg4_aac.pce.clone());

            //self.mpeg4_aac.npce = cur_bits_data
        }

        Ok(0)
    }

    pub fn get_audio_object_type(&mut self) -> Result<u8, MpegAacError> {
        let mut audio_object_type: u64;

        audio_object_type = self.bits_data.read_n_bits(5)?;

        if 31 == audio_object_type {
            audio_object_type = 32 + self.bits_data.read_n_bits(6)?;
        }

        Ok(audio_object_type as u8)
    }

    pub fn get_sampling_frequency(&mut self) -> Result<u8, MpegAacError> {
        let mut sampling_frequency_index: u64;

        sampling_frequency_index = self.bits_data.read_n_bits(4)?;

        if sampling_frequency_index == 0x0F {
            sampling_frequency_index = self.bits_data.read_n_bits(24)?;
        }

        Ok(sampling_frequency_index as u8)
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
