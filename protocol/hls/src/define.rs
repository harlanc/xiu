use libflv::demuxer::FlvDemuxerAudioData;
use libflv::demuxer::FlvDemuxerVideoData;

pub const HLS_DURATION: u8 = 10;

pub enum FlvDemuxerData {
    Video { data: FlvDemuxerVideoData },
    Audio { data: FlvDemuxerAudioData },
    None,
}
