use {
    super::{
        bitvec as mpeg4bitvec,
        bitvec::Mpeg4BitVec,
        errors::{MpegAacError, MpegAacErrorValue},
    },
    bytes::BytesMut,
    bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter},
};

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

// pub struct Mpeg4Bits {
//     data: BytesMut,
//     size: usize,
//     bits: usize,
//     error: u32,
// }

// impl Mpeg4Bits {
//     pub fn default() -> Self {
//         Self {
//             data: BytesMut::new(),
//             size: 0,
//             bits: 0,
//             error: 0,
//         }
//     }
// }

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
        let byte_0 = self.bytes_reader.advance_u8()?;
        self.mpeg4_aac.profile = (byte_0 >> 3) & 0x1F;

        let byte_1 = self.bytes_reader.advance_u8()?;
        self.mpeg4_aac.sampling_frequency_index = ((byte_0 & 0x07) << 1) | ((byte_1 >> 7) & 0x01);
        self.mpeg4_aac.channel_configuration = (byte_1 >> 3) & 0x0F;
        self.mpeg4_aac.channels = self.mpeg4_aac.channel_configuration;
        self.mpeg4_aac.sampling_frequency =
            AAC_FREQUENCE[self.mpeg4_aac.sampling_frequency_index as usize];

        // if self.bytes_reader.len() > 2 {
        //     return self.audio_specific_config_load2();
        // }

        self.bytes_reader.read_u8()?;
        self.bytes_reader.read_u8()?;

        self.bytes_reader.extract_remaining_bytes();

        Ok(())
    }

    pub fn audio_specific_config_load2(&mut self) -> Result<(), MpegAacError> {
        let remain_bytes = self.bytes_reader.extract_remaining_bytes();
        self.bits_data.extend_from_bytesmut(remain_bytes);

        self.mpeg4_aac.profile = self.get_audio_object_type()?;
        self.mpeg4_aac.sampling_frequency_index = self.get_sampling_frequency()?;
        self.mpeg4_aac.channel_configuration = self.bits_data.read_n_bits(4)? as u8;

        let mut extension_audio_object_type: u8;
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

        let ep_config: u64;

        match self.mpeg4_aac.profile {
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 17 | 19 | 20 | 21 | 22 | 23 => {
                self.ga_specific_config_load()?;
            }
            8 => {
                self.celp_specific_config_load()?;
            }
            _ => {}
        }

        match self.mpeg4_aac.profile {
            17 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 39 => {
                ep_config = self.bits_data.read_n_bits(2)?;

                match ep_config {
                    2 | 3 => {
                        return Err(MpegAacError {
                            value: MpegAacErrorValue::ShouldNotComeHere,
                        });
                    }

                    _ => {}
                }
            }
            _ => {}
        }

        let mut sync_extension_type: u64;

        if 5 != extension_audio_object_type && self.bits_data.len() >= 16 {
            sync_extension_type = self.bits_data.read_n_bits(11)?;

            if 0x2B7 == sync_extension_type {
                extension_audio_object_type = self.get_audio_object_type()?;

                match extension_audio_object_type {
                    5 => {
                        self.mpeg4_aac.sbr = self.bits_data.read_n_bits(1)? as usize;
                        if self.mpeg4_aac.sbr > 0 {
                            extension_sampling_frequency_index = self.get_sampling_frequency()?;
                            if self.bits_data.len() >= 12 {
                                sync_extension_type = self.bits_data.read_n_bits(11)?;
                                if 0x548 == sync_extension_type {
                                    self.mpeg4_aac.ps = self.bits_data.read_n_bits(1)? as usize;
                                }
                            }
                        }
                    }
                    22 => {
                        self.mpeg4_aac.sbr = self.bits_data.read_n_bits(1)? as usize;

                        if self.mpeg4_aac.sbr > 0 {
                            extension_sampling_frequency_index = self.get_sampling_frequency()?;
                        }

                        extension_channel_configuration = self.bits_data.read_n_bits(4)? as u8;
                    }

                    _ => {}
                }
            }
        }

        self.bits_data
            .bits_aligment(8, mpeg4bitvec::BitVectorOpType::Read)?;

        log::trace!(
            "remove warnings: {} {} {}",
            extension_audio_object_type,
            extension_sampling_frequency_index,
            extension_channel_configuration
        );

        Ok(())
    }

    pub fn celp_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        let excitation_mode: u64;

        if self.bits_data.read_n_bits(1)? > 0 {
            excitation_mode = self.bits_data.read_n_bits(1)?;
            self.bits_data.read_n_bits(1)?;
            self.bits_data.read_n_bits(1)?;

            if excitation_mode == 1 {
                self.bits_data.read_n_bits(3)?;
            } else if excitation_mode == 0 {
                self.bits_data.read_n_bits(5)?;
                self.bits_data.read_n_bits(2)?;
                self.bits_data.read_n_bits(1)?;
            }
        } else {
            if self.bits_data.read_n_bits(1)? > 0 {
                self.bits_data.read_n_bits(2)?;
            } else {
                self.bits_data.read_n_bits(2)?;
            }
        }

        Ok(())
    }
    pub fn ga_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        let extension_flag: u64;
        self.bits_data.read_n_bits(1)?;

        if self.bits_data.read_n_bits(1)? > 0 {
            self.bits_data.read_n_bits(14)?;
        }
        extension_flag = self.bits_data.read_n_bits(1)?;

        if 0 == self.mpeg4_aac.channel_configuration {
            self.pce_load()?;
        }

        if self.mpeg4_aac.profile == 6 || self.mpeg4_aac.profile == 20 {
            self.bits_data.read_n_bits(3)?;
        }

        if extension_flag > 0 {
            match self.mpeg4_aac.profile {
                22 => {
                    self.bits_data.read_n_bits(5)?;
                    self.bits_data.read_n_bits(11)?;
                }
                17 | 19 | 20 | 23 => {
                    self.bits_data.read_n_bits(1)?;
                    self.bits_data.read_n_bits(1)?;
                    self.bits_data.read_n_bits(1)?;
                }
                _ => {}
            }

            self.bits_data.read_n_bits(1)?;
        }

        Ok(())
    }

    pub fn pce_load(&mut self) -> Result<u8, MpegAacError> {
        let mut cpe: u64 = 0;
        let mut tag: u64 = 0;
        let element_instance_tag: u64;
        let object_type: u64;
        let sampling_frequency_index: u64;
        let num_front_channel_elements: u64;
        let num_side_channel_elements: u64;
        let num_back_channel_elements: u64;
        let num_lfe_channel_elements: u64;
        let num_assoc_data_elements: u64;
        let num_valid_cc_elements: u64;
        let comment_field_bytes: u64;

        let mut pce_bits_vec = Mpeg4BitVec::new();
        pce_bits_vec.extend_from_bytesmut(self.mpeg4_aac.pce.clone());

        self.mpeg4_aac.channels = 0;

        element_instance_tag =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        object_type = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 2)?;
        sampling_frequency_index =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        num_front_channel_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        num_side_channel_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        num_back_channel_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        num_lfe_channel_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 2)?;
        num_assoc_data_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 3)?;
        num_valid_cc_elements =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;

        for _ in 0..3 {
            if mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 1)? > 0 {
                mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
            }
        }

        for _ in 0..num_front_channel_elements {
            cpe = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 1)?;
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_side_channel_elements {
            cpe = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 1)?;
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_back_channel_elements {
            cpe = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 1)?;
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_lfe_channel_elements {
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
            self.mpeg4_aac.channels += 1;
        }

        for _ in 0..num_assoc_data_elements {
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        }

        for _ in 0..num_valid_cc_elements {
            cpe = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 1)?;
            tag = mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 4)?;
        }

        pce_bits_vec.bits_aligment(8, mpeg4bitvec::BitVectorOpType::Write)?;
        self.bits_data
            .bits_aligment(8, mpeg4bitvec::BitVectorOpType::Read)?;

        comment_field_bytes =
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 8)?;

        for _ in 0..comment_field_bytes {
            mpeg4bitvec::mpeg4_bits_copy(&mut pce_bits_vec, &mut self.bits_data, 8)?;
        }

        let rv = (pce_bits_vec.write_offset + 7) / 8;

        log::trace!(
            "remove warnings: {} {} {} {} {}",
            tag,
            element_instance_tag,
            object_type,
            sampling_frequency_index,
            cpe
        );

        Ok(rv as u8)
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
            .write(&self.bytes_reader.extract_remaining_bytes()[..])?;

        Ok(())
    }
}
