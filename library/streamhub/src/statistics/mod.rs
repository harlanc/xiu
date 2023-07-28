pub mod avstatistics;

use {
    super::stream::StreamIdentifier,
    serde::Serialize,
    xflv::define::{AacProfile, AvcCodecId, AvcLevel, AvcProfile, SoundFormat},
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoInfo {
    codec: AvcCodecId,
    profile: AvcProfile,
    level: AvcLevel,
    width: u32,
    height: u32,
    #[serde(rename = "bitrate(kbits/s)")]
    bitrate: f32,
    frame_rate: usize,
    gop: usize,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct AudioInfo {
    sound_format: SoundFormat,
    profile: AacProfile,
    samplerate: u32,
    channels: u8,
    #[serde(rename = "bitrate(kbits/s)")]
    bitrate: f32,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct StreamStatistics {
    identifier: StreamIdentifier,
    pub video: VideoInfo,
    pub audio: AudioInfo,
}

impl StreamStatistics {
    pub fn new(identifier: StreamIdentifier) -> Self {
        Self {
            identifier,
            ..Default::default()
        }
    }
}
