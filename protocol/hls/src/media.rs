use core::time;

use super::errors::MediaError;
use byteorder::BigEndian;
use libflv::define::FlvDemuxerData;
use libflv::demuxer::FlvAudioDemuxer;
use libflv::demuxer::FlvDemuxerAudioData;
use libflv::demuxer::FlvDemuxerVideoData;
use libflv::demuxer::FlvVideoDemuxer;
use libflv::muxer::HEADER_LENGTH;
use networkio::bytes_writer::BytesWriter;
use rtmp::amf0::amf0_writer::Amf0Writer;
use rtmp::cache::metadata::MetaData;
use rtmp::session::common::SessionInfo;
use rtmp::session::define::SessionSubType;
use rtmp::session::errors::SessionError;
use rtmp::session::errors::SessionErrorValue;
use rtmp::utils::print;
use {
    bytes::BytesMut,
    rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventProducer,
    },
    std::time::Duration,
    tokio::{
        sync::{mpsc, oneshot, Mutex},
        time::sleep,
    },
};

use libflv::define::frame_type;

pub struct Media {
    video_demuxer: FlvVideoDemuxer,
    audio_demuxer: FlvAudioDemuxer,

    pts: u64,
    last_dts: u64,

    duration: u64,
    need_new_segment: bool,
}

impl Media {
    pub fn new(duration: u64) -> Self {
        Self {
            video_demuxer: FlvVideoDemuxer::new(),
            audio_demuxer: FlvAudioDemuxer::new(),

            pts: 0,
            last_dts: 0,
            duration,
            need_new_segment: false,
        }
    }

    pub fn demux(&mut self, data: ChannelData) -> Result<(), MediaError> {
        let cur_dts: u32;
        match data {
            ChannelData::Audio { timestamp, data } => {
                let audio_data = self.audio_demuxer.demuxer(timestamp, data)?;
            }
            ChannelData::Video { timestamp, data } => {
                let video_data = self.video_demuxer.demuxer(timestamp, data)?;

                if video_data.has_data
                    && video_data.dts - self.last_dts >= self.duration
                    && video_data.frame_type == frame_type::KEY_FRAME
                {
                    self.need_new_segment = true
                }
            }
            ChannelData::MetaData { timestamp, data } => {}
        }

        Ok(())
    }
}
