use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum RtspCodecId {
    #[default]
    H264,
    H265,
    AAC,
    G711A,
}

lazy_static! {
    pub static ref RTSP_CODEC_ID_2_NAME: HashMap<RtspCodecId, &'static str> = {
        let mut m = HashMap::new();
        m.insert(RtspCodecId::H264, "h264");
        m.insert(RtspCodecId::H265, "h265");
        m.insert(RtspCodecId::AAC, "mpeg4-generic");
        m.insert(RtspCodecId::G711A, "pcma");
        m
    };
    pub static ref RTSP_CODEC_NAME_2_ID: HashMap<&'static str, RtspCodecId> = {
        let mut m = HashMap::new();
        m.insert("h264", RtspCodecId::H264);
        m.insert("h265", RtspCodecId::H265);
        m.insert("mpeg4-generic", RtspCodecId::AAC);
        m.insert("pcma", RtspCodecId::G711A);
        m
    };
}
#[derive(Debug, Clone, Default)]
pub struct RtspCodecInfo {
    pub codec_id: RtspCodecId,
    pub payload_type: u8,
    pub sample_rate: u32,
    pub channel_count: u8,
}
