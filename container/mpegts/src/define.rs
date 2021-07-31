pub mod epat_pid {
    pub const PAT_TID_PAS: u8 = 0x00;
    pub const PAT_TID_CAS: u8 = 0x01; // conditional_access_section(CA_section)
    pub const PAT_TID_PMS: u8 = 0x02; // TS_program_map_section
}

pub mod ts {
    pub const TS_PACKET_SIZE: u8 = 188;
}

pub mod epsi_stream_type {
    pub const PSI_STREAM_PRIVATE_DATA: u8 = 0x06;
    pub const PSI_STREAM_H264: u8 = 0x1b; // H.264
    pub const PSI_STREAM_AUDIO_OPUS: u8 = 0x9c;
}

pub const AF_FLAG_PCR: u8 = 0x10;
pub const AF_FLAG_RANDOM_ACCESS_INDICATOR: u8 = 0x40;
pub const PTS_NO_VALUE: i64 = i64::MIN; //(int64_t)0x8000000000000000L

pub const TS_HEADER_LEN: u8 = 4; // 1-bytes sync byte + 2-bytes PID + 1-byte CC
pub const PES_HEADER_LEN: u8 = 6; // 3-bytes packet_start_code_prefix + 1-byte stream_id + 2-bytes PES_packet_length

pub const TS_PAYLOAD_UNIT_START_INDICATOR: u8 = 0x40;

pub const TS_PACKET_SIZE: usize = 188;
