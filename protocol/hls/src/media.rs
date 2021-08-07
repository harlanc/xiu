use core::time;

use super::define::FlvDemuxerData;
use super::errors::MediaError;
use byteorder::BigEndian;
use bytes::BufMut;
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
use libmpegts::define::epsi_stream_type;
use libmpegts::define::MPEG_FLAG_IDR_FRAME;
use libmpegts::ts_muxer::TsMuxer;

pub struct Media {
    video_demuxer: FlvVideoDemuxer,
    audio_demuxer: FlvAudioDemuxer,

    ts_muxer: TsMuxer,

    pts: i64,
    last_ts_dts: i64,
    last_ts_pts: i64,

    duration: i64,
    need_new_segment: bool,

    video_pid: u16,
    audio_pid: u16,
}

impl Media {
    pub fn new(duration: i64) -> Self {
        let mut ts_muxer = TsMuxer::new();
        let video_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_H264, BytesMut::new())
            .unwrap();
        let audio_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_AAC, BytesMut::new())
            .unwrap();
        Self {
            video_demuxer: FlvVideoDemuxer::new(),
            audio_demuxer: FlvAudioDemuxer::new(),

            ts_muxer,

            pts: 0,
            last_ts_dts: 0,
            last_ts_pts: 0,
            duration,
            need_new_segment: false,

            video_pid,
            audio_pid,
        }
    }

    pub fn demux(&mut self, data: ChannelData) -> Result<(), MediaError> {
        let flv_demux_data: FlvDemuxerData;

        match data {
            ChannelData::Audio { timestamp, data } => {
                let audio_data = self.audio_demuxer.demux(timestamp, data)?;
                flv_demux_data = FlvDemuxerData::Audio { data: audio_data };
            }
            ChannelData::Video { timestamp, data } => {
                let video_data = self.video_demuxer.demux(timestamp, data)?;
                flv_demux_data = FlvDemuxerData::Video { data: video_data };
            }
            ChannelData::MetaData { timestamp, data } => {
                flv_demux_data = FlvDemuxerData::None;
            }
        }

        self.process_media_data(&flv_demux_data)?;

        Ok(())
    }

    pub fn process_media_data(
        &mut self,
        flv_demux_data: &FlvDemuxerData,
    ) -> Result<(), MediaError> {
        self.need_new_segment = false;

        let mut pid: u16 = 0;
        let mut pts: i64 = 0;
        let mut dts: i64 = 0;
        let mut flags: u16 = 0;
        let mut payload: BytesMut = BytesMut::new();

        match flv_demux_data {
            FlvDemuxerData::Video { data } => {
                if !data.has_data {
                    return Ok(());
                }

                pts = data.pts * 90;
                dts = data.dts * 90;
                pid = self.video_pid;
                payload.extend_from_slice(&data.data[..]);

                if data.frame_type == frame_type::KEY_FRAME {
                    flags = MPEG_FLAG_IDR_FRAME;
                    if data.dts - self.last_ts_dts >= self.duration {
                        self.need_new_segment = true;
                    }
                }
            }
            FlvDemuxerData::Audio { data } => {
                if !data.has_data {
                    return Ok(());
                }

                pts = data.pts * 90;
                dts = data.dts * 90;
                pid = self.audio_pid;
                payload.extend_from_slice(&data.data[..]);
            }
            _ => return Ok(()),
        }

        if self.need_new_segment {
            self.ts_muxer.reset();
            self.last_ts_dts = dts;
            self.last_ts_pts = pts;
        }

        self.ts_muxer.write(pid, pts, dts, flags, payload)?;

        Ok(())
    }
}
