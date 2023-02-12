use {
    super::{crc32, define::epat_pid, errors::MpegTsError, pmt},
    byteorder::{BigEndian, LittleEndian},
    bytes::BytesMut,
    bytesio::bytes_writer::BytesWriter,
};

#[derive(Debug, Clone)]
pub struct Pat {
    transport_stream_id: u16,
    version_number: u8, //5bits
    //continuity_counter: u8, //s4 bits

    //pub pmt_count: usize,
    pub pmt: Vec<pmt::Pmt>,
}

impl Default for Pat {
    fn default() -> Self {
        Self::new()
    }
}

impl Pat {
    pub fn new() -> Self {
        Self {
            transport_stream_id: 1,
            version_number: 0,
            //continuity_counter: 0,
            //pmt_count: 0,
            pmt: Vec::new(),
        }
    }
}
pub struct PatMuxer {
    pub bytes_writer: BytesWriter,
}

impl Default for PatMuxer {
    fn default() -> Self {
        Self::new()
    }
}
//ITU-T H.222.0
impl PatMuxer {
    pub fn new() -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
        }
    }

    pub fn write(&mut self, pat: Pat) -> Result<BytesMut, MpegTsError> {
        /*table id*/
        self.bytes_writer.write_u8(epat_pid::PAT_TID_PAS as u8)?;

        /*section length*/
        let length = pat.pmt.len() as u16 * 4 + 5 + 4;
        self.bytes_writer.write_u16::<BigEndian>(0xb000 | length)?;
        /*transport_stream_id*/
        self.bytes_writer
            .write_u16::<BigEndian>(pat.transport_stream_id)?;
        /*version_number*/
        self.bytes_writer
            .write_u8(0xC1 | (pat.version_number << 1))?;

        /*section_number*/
        /*last_section_number*/
        self.bytes_writer.write_u16::<BigEndian>(0x00)?;

        for ele in &pat.pmt {
            /*program number*/
            self.bytes_writer
                .write_u16::<BigEndian>(ele.program_number)?;
            /*PID*/
            self.bytes_writer.write_u16::<BigEndian>(0xE000 | ele.pid)?;
        }

        /*crc32*/
        let crc32_value = crc32::gen_crc32(0xffffffff, self.bytes_writer.get_current_bytes());
        self.bytes_writer.write_u32::<LittleEndian>(crc32_value)?;

        // let mut test = BytesWriter::new();
        // test.write_u32::<LittleEndian>(crc32_value)?;
        // let a0 = test.get(0).unwrap().clone();
        // let aa0 = crc32_value & 0xFF;
        // let b0 = test.get(1).unwrap().clone();
        // let bb0 = (crc32_value >> 8) & 0xFF;
        // let c0 = test.get(2).unwrap().clone();
        // let cc0 = (crc32_value >> 16) & 0xFF;
        // let d0 = test.get(3).unwrap().clone();
        // let dd0 = (crc32_value >> 24) & 0xFF;

        Ok(self.bytes_writer.extract_current_bytes())
    }
}
