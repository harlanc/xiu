use {
    super::{define::h264_nal_type, errors::MpegAvcError},
    byteorder::BigEndian,
    bytes::BytesMut,
    bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter},
    std::vec::Vec,
};

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
    nalu_length: u8,

    nb_sps: u8,
    nb_pps: u8,

    sps: Vec<Sps>,
    pps: Vec<Pps>,

    sps_annexb_data: BytesWriter, // pice together all the sps data
    pps_annexb_data: BytesWriter, // pice together all the pps data

    //extension
    chroma_format_idc: u8,
    bit_depth_luma_minus8: u8,
    bit_depth_chroma_minus8: u8,
    // data: Vec<u8>, //[u8; 4 * 1024],
    // off: i32,
}

pub fn print(data: BytesMut) {
    print!("==========={}\n", data.len());
    let mut idx = 0;
    for i in data {
        print!("{:02X} ", i);
        idx = idx + 1;
        match idx % 16 {
            0 => {
                print!("\n")
            }
            _ => {}
        }
    }

    print!("===========\n")
}

impl Mpeg4Avc {
    pub fn default() -> Self {
        Self {
            profile: 0,
            compatibility: 0,
            level: 0,
            nalu_length: 0,
            nb_pps: 0,
            nb_sps: 0,

            sps: Vec::new(),
            pps: Vec::new(),

            sps_annexb_data: BytesWriter::new(),
            pps_annexb_data: BytesWriter::new(),

            chroma_format_idc: 0,
            bit_depth_chroma_minus8: 0,
            bit_depth_luma_minus8: 0,
        }
    }
}

pub struct Mpeg4AvcProcessor {
    pub bytes_reader: BytesReader,
    pub bytes_writer: BytesWriter,
    pub mpeg4_avc: Mpeg4Avc,
}

impl Mpeg4AvcProcessor {
    pub fn new() -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bytes_writer: BytesWriter::new(),
            mpeg4_avc: Mpeg4Avc::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bytes_reader.extend_from_slice(&data[..]);
    }

    pub fn clear_sps_data(&mut self) {
        self.mpeg4_avc.sps.clear();
        self.mpeg4_avc.sps_annexb_data.clear();
    }

    pub fn clear_pps_data(&mut self) {
        self.mpeg4_avc.pps.clear();
        self.mpeg4_avc.pps_annexb_data.clear();
    }

    pub fn decoder_configuration_record_load(&mut self) -> Result<(), MpegAvcError> {
        /*version */
        self.bytes_reader.read_u8()?;
        /*avc profile*/
        self.mpeg4_avc.profile = self.bytes_reader.read_u8()?;
        /*avc compatibility*/
        self.mpeg4_avc.compatibility = self.bytes_reader.read_u8()?;
        /*avc level*/
        self.mpeg4_avc.level = self.bytes_reader.read_u8()?;
        /*nalu length*/
        self.mpeg4_avc.nalu_length = self.bytes_reader.read_u8()? & 0x03 + 1;

        /*number of SPS NALUs */
        self.mpeg4_avc.nb_sps = self.bytes_reader.read_u8()? & 0x1F;

        if self.mpeg4_avc.nb_sps > 0 {
            self.clear_sps_data();
        }

        for i in 0..self.mpeg4_avc.nb_sps as usize {
            /*SPS size*/
            let sps_data_size = self.bytes_reader.read_u16::<BigEndian>()?;
            let sps_data = Sps {
                size: sps_data_size,
                /*SPS data*/
                data: self.bytes_reader.read_bytes(sps_data_size as usize)?,
            };

            self.mpeg4_avc.sps.push(sps_data);
            self.mpeg4_avc.sps_annexb_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .sps_annexb_data
                .write(&self.mpeg4_avc.sps[i].data[..])?;
        }

        /*number of PPS NALUs*/
        self.mpeg4_avc.nb_pps = self.bytes_reader.read_u8()?;

        if self.mpeg4_avc.nb_pps > 0 {
            self.clear_pps_data();
        }

        for i in 0..self.mpeg4_avc.nb_pps as usize {
            let pps_data_size = self.bytes_reader.read_u16::<BigEndian>()?;
            let pps_data = Pps {
                size: pps_data_size,
                data: self.bytes_reader.read_bytes(pps_data_size as usize)?,
            };

            self.mpeg4_avc.pps.push(pps_data);

            self.mpeg4_avc.pps_annexb_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .pps_annexb_data
                .write(&self.mpeg4_avc.pps[i].data[..])?;
        }

        Ok(())
    }
    //https://stackoverflow.com/questions/28678615/efficiently-insert-or-replace-multiple-elements-in-the-middle-or-at-the-beginnin
    pub fn h264_mp4toannexb(&mut self) -> Result<(), MpegAvcError> {
        let mut sps_pps_flag = false;

        while self.bytes_reader.len() > 0 {
            let size = self.get_nalu_size()?;
            let nalu_type = self.bytes_reader.advance_u8()? & 0x1f;

            match nalu_type {
                h264_nal_type::H264_NAL_PPS | h264_nal_type::H264_NAL_SPS => {
                    sps_pps_flag = true;
                }

                h264_nal_type::H264_NAL_IDR => {
                    if !sps_pps_flag {
                        sps_pps_flag = true;

                        self.bytes_writer
                            .prepend(&self.mpeg4_avc.pps_annexb_data.get_current_bytes()[..])?;
                        self.bytes_writer
                            .prepend(&self.mpeg4_avc.sps_annexb_data.get_current_bytes()[..])?;
                    }
                }

                _ => {}
            }

            self.bytes_writer.write(&H264_START_CODE)?;
            let data = self.bytes_reader.read_bytes(size as usize)?;
            self.bytes_writer.write(&data[..])?;

            //print(self.bytes_writer.get_current_bytes());
        }

        Ok(())
    }

    pub fn get_nalu_size(&mut self) -> Result<u32, MpegAvcError> {
        let mut size: u32 = 0;

        for _ in 0..self.mpeg4_avc.nalu_length {
            size = self.bytes_reader.read_u8()? as u32 + (size << 8);
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
            .write_u8((self.mpeg4_avc.nalu_length - 1) | 0xFC)?;

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
