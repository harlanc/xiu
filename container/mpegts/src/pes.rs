use bytes::BytesMut;

pub struct Pes {
    program_number: u16,
    pid: u16,
    stream_id: u8,
    codec_id: u8,
    continuity_counter: u8,
    esinfo: BytesMut,
    esinfo_length: u16,
    packet_length: u32,

    reserved10: u8,               //2
    pes_scrambling_control: u8,   //2
    pes_priority: u8,             //1
    data_alignment_indicator: u8, //1
    copyright: u8,                //1
    original_or_copy: u8,         //1

    pts_dts_flags: u8,             //2
    escr_flag: u8,                 //1
    es_rate_flag: u8,              //1
    dsm_trick_mode_flag: u8,       //1
    additional_copy_info_flag: u8, //1
    pes_crc_flag: u8,              //1
    pes_extension_flag: u8,        //1
    pes_header_data_length: u8,    //8

    pts: u64,
    dts: u64,
    escr_base: u64,
    escr_extension: u32,
    es_rate: u32,
    packet: Packet,
}

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
