pub mod epat_pid {
    pub const PAT_TID_PAS: u16 = 0x00;
    pub const PAT_TID_CAS: u16 = 0x01; // conditional_access_section(CA_section)
    pub const PAT_TID_PMS: u16 = 0x02; // TS_program_map_section
}

pub mod ts {
    pub const TS_PACKET_SIZE: u8 = 188;
}

pub mod epsi_stream_type {
    pub const PSI_STREAM_MP3: u8 = 0x04; // ISO/IEC 13818-3 Audio
    pub const PSI_STREAM_PRIVATE_DATA: u8 = 0x06;
    pub const PSI_STREAM_H264: u8 = 0x1b; // H.264
    pub const PSI_STREAM_H265: u8 = 0x24;
    pub const PSI_STREAM_AAC: u8 = 0x0f;
    pub const PSI_STREAM_MPEG4_AAC: u8 = 0x1c;
    pub const PSI_STREAM_AUDIO_G711A: u8 = 0x90; // GBT 25724-2010 SVAC(2014)
    pub const PSI_STREAM_AUDIO_G711U: u8 = 0x91;
    pub const PSI_STREAM_AUDIO_OPUS: u8 = 0x9c;
}

pub mod epes_stream_id {

    pub const PES_SID_EXTENSION: u8 = 0xB7; // PS system_header extension(p81)
    pub const PES_SID_END: u8 = 0xB9; // MPEG_program_end_code
    pub const PES_SID_START: u8 = 0xBA; // Pack start code
    pub const PES_SID_SYS: u8 = 0xBB; // System header start code

    pub const PES_SID_PSM: u8 = 0xBC; // program_stream_map
    pub const PES_SID_PRIVATE_1: u8 = 0xBD; // private_stream_1
    pub const PES_SID_PADDING: u8 = 0xBE; // padding_stream
    pub const PES_SID_PRIVATE_2: u8 = 0xBF; // private_stream_2
    pub const PES_SID_AUDIO: u8 = 0xC0; // ISO/IEC 13818-3/11172-3/13818-7/14496-3 audio stream '110x xxxx'
    pub const PES_SID_VIDEO: u8 = 0xE0; // H.262 | H.264 | H.265 | ISO/IEC 13818-2/11172-2/14496-2/14496-10 video stream '1110 xxxx'
    pub const PES_SID_ECM: u8 = 0xF0; // ECM_stream
    pub const PES_SID_EMM: u8 = 0xF1; // EMM_stream
    pub const PES_SID_DSMCC: u8 = 0xF2; // H.222.0 | ISO/IEC 13818-1/13818-6_DSMCC_stream
    pub const PES_SID_13522: u8 = 0xF3; // ISO/IEC_13522_stream
    pub const PES_SID_H222_A: u8 = 0xF4; // Rec. ITU-T H.222.1 type A
    pub const PES_SID_H222_B: u8 = 0xF5; // Rec. ITU-T H.222.1 type B
    pub const PES_SID_H222_C: u8 = 0xF6; // Rec. ITU-T H.222.1 type C
    pub const PES_SID_H222_D: u8 = 0xF7; // Rec. ITU-T H.222.1 type D
    pub const PES_SID_H222_E: u8 = 0xF8; // Rec. ITU-T H.222.1 type E
    pub const PES_SID_ANCILLARY: u8 = 0xF9; // ancillary_stream
    pub const PES_SID_MPEG4_SL: u8 = 0xFA; // ISO/IEC 14496-1_SL_packetized_stream
    pub const PES_SID_MPEG4_FLEX: u8 = 0xFB; // ISO/IEC 14496-1_FlexMux_stream
    pub const PES_SID_META: u8 = 0xFC; // metadata stream
    pub const PES_SID_EXTEND: u8 = 0xFD; // extended_stream_id
    pub const PES_SID_RESERVED: u8 = 0xFE; // reserved data stream
    pub const PES_SID_PSD: u8 = 0xFF; // program_stream_directory
}

pub const AF_FLAG_PCR: u8 = 0x10;
pub const AF_FLAG_RANDOM_ACCESS_INDICATOR: u8 = 0x40;
pub const PTS_NO_VALUE: i64 = i64::MIN; //(int64_t)0x8000000000000000L

pub const TS_HEADER_LEN: u8 = 4; // 1-bytes sync byte + 2-bytes PID + 1-byte CC
pub const PES_HEADER_LEN: u8 = 6; // 3-bytes packet_start_code_prefix + 1-byte stream_id + 2-bytes PES_packet_length

pub const TS_PAYLOAD_UNIT_START_INDICATOR: u8 = 0x40;

pub const TS_PACKET_SIZE: usize = 188;

pub const MPEG_FLAG_IDR_FRAME: u16 = 0x0001;
pub const MPEG_FLAG_H264_H265_WITH_AUD: u16 = 0x8000;

pub const PAT_PERIOD: u64 = 400 * 90;
