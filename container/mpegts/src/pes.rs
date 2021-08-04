use bytes::BytesMut;
use networkio::bytes_writer::BytesWriter;

#[derive(Debug, Clone)]
pub struct Pes {
    pub program_number: u16,
    pub pid: u16,
    pub stream_id: u8,
    pub codec_id: u8,
    pub continuity_counter: u8,
    pub esinfo: BytesMut,
    pub esinfo_length: usize,
    packet_length: u32,

    reserved10: u8,                   //2
    pes_scrambling_control: u8,       //2
    pes_priority: u8,                 //1
    pub data_alignment_indicator: u8, //1
    copyright: u8,                    //1
    original_or_copy: u8,             //1

    pts_dts_flags: u8,             //2
    escr_flag: u8,                 //1
    es_rate_flag: u8,              //1
    dsm_trick_mode_flag: u8,       //1
    additional_copy_info_flag: u8, //1
    pes_crc_flag: u8,              //1
    pes_extension_flag: u8,        //1
    pes_header_data_length: u8,    //8

    pub pts: i64,
    pub dts: i64,
    escr_base: u64,
    escr_extension: u32,
    es_rate: u32,
    packet: Packet,
}

impl Pes {
    pub fn default() -> Self {
        Self {
            program_number: 0,
            pid: 0,
            stream_id: 0,
            codec_id: 0,
            continuity_counter: 0,
            esinfo: BytesMut::new(),
            esinfo_length: 0,
            packet_length: 0,

            reserved10: 0,               //2
            pes_scrambling_control: 0,   //2
            pes_priority: 0,             //1
            data_alignment_indicator: 0, //1
            copyright: 0,                //1
            original_or_copy: 0,         //1

            pts_dts_flags: 0,             //2
            escr_flag: 0,                 //1
            es_rate_flag: 0,              //1
            dsm_trick_mode_flag: 0,       //1
            additional_copy_info_flag: 0, //1
            pes_crc_flag: 0,              //1
            pes_extension_flag: 0,        //1
            pes_header_data_length: 0,    //8

            pts: 0,
            dts: 0,
            escr_base: 0,
            escr_extension: 0,
            es_rate: 0,
            packet: Packet::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Packet {
    stream_id: u8,
    codec_id: u8,

    flags: i32,
    pts: i64,
    dts: i64,
    data: BytesMut,
    size: usize,
    capacity: usize,
}

impl Packet {
    pub fn default() -> Self {
        Self {
            stream_id: 0,
            codec_id: 0,

            flags: 0,
            pts: 0,
            dts: 0,
            data: BytesMut::new(),
            size: 0,
            capacity: 0,
        }
    }
}

pub struct PatWriter {
    pub bytes_writer: BytesWriter,
}

// impl PatWriter {
