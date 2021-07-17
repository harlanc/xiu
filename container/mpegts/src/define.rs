pub mod epat_pid {
    pub const PAT_TID_PAS: u8 = 0x00;
    pub const PAT_TID_CAS: u8 = 0x01; // conditional_access_section(CA_section)
    pub const PAT_TID_PMS: u8 = 0x02; // TS_program_map_section
}

pub mod ts {
    pub const TS_PACKET_SIZE: u8 = 188;
}

pub mod epsi_stream_type {

    pub const PSI_STREAM_AUDIO_OPUS: u8 = 0x9c;
    pub const PSI_STREAM_PRIVATE_DATA: u8 = 0x06;
}
