pub enum TagType {
    VIDEO,
    AUDIO,
}

pub mod sound_format {
    pub const AAC: u8 = 10;
}

pub mod aac_packet_type {
    pub const AAC_SEQHDR: u8 = 0;
    pub const AAC_RAW: u8 = 1;
}

pub mod avc_packet_type {
    pub const AVC_SEQHDR: u8 = 0;
    pub const AVC_NALU: u8 = 1;
    pub const AVC_EOS: u8 = 2;
}

pub mod frame_type {
    /*
        1: keyframe (for AVC, a seekable frame)
        2: inter frame (for AVC, a non- seekable frame)
        3: disposable inter frame (H.263 only)
        4: generated keyframe (reserved for server use only)
        5: video info/command frame
    */
    pub const KEY_FRAME: u8 = 1;
    pub const INTER_FRAME: u8 = 2;
}
