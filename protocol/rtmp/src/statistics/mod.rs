pub mod avstatistics;

use {
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

    bitrate: f32,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct StreamStatistics {
    pub video: VideoInfo,
    pub audio: AudioInfo,
}
