use core::time;

use super::define::FlvDemuxerData;
use super::errors::MediaError;
use super::m3u8::M3u8;
use super::ts::Ts;
use byteorder::BigEndian;
use bytes::BufMut;
use libflv::demuxer::FlvAudioTagDemuxer;
use libflv::demuxer::FlvDemuxerAudioData;
use libflv::demuxer::FlvDemuxerVideoData;
use libflv::demuxer::FlvVideoTagDemuxer;

use libflv::define::FlvData;
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
    video_demuxer: FlvVideoTagDemuxer,
    audio_demuxer: FlvAudioTagDemuxer,

    ts_muxer: TsMuxer,

    last_ts_dts: i64,
    last_ts_pts: i64,

    last_dts: i64,
    last_pts: i64,

    duration: i64,
    need_new_segment: bool,

    video_pid: u16,
    audio_pid: u16,

    m3u8_handler: M3u8,
    ts_handler: Ts,
}

impl Media {
    pub fn new(duration: i64) -> Self {
        let mut ts_muxer = TsMuxer::new();
        let audio_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_AAC, BytesMut::new())
            .unwrap();
        let video_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_H264, BytesMut::new())
            .unwrap();

        Self {
            video_demuxer: FlvVideoTagDemuxer::new(),
            audio_demuxer: FlvAudioTagDemuxer::new(),

            ts_muxer,

            last_ts_dts: 0,
            last_ts_pts: 0,

            last_dts: 0,
            last_pts: 0,

            duration,
            need_new_segment: false,

            video_pid,
            audio_pid,
            m3u8_handler: M3u8::new(duration, 3, String::from("test.m3u8")),
            ts_handler: Ts::new(),
        }
    }

    pub fn process_flv_data(&mut self, data: FlvData) -> Result<(), MediaError> {
        let flv_demux_data: FlvDemuxerData;

        match data {
            FlvData::Audio { timestamp, data } => {
                let audio_data = self.audio_demuxer.demux(timestamp, data)?;
                flv_demux_data = FlvDemuxerData::Audio { data: audio_data };
            }
            FlvData::Video { timestamp, data } => {
                let video_data = self.video_demuxer.demux(timestamp, data)?;
                flv_demux_data = FlvDemuxerData::Video { data: video_data };
            }
            FlvData::MetaData { timestamp, data } => return Ok(()),
        }

        self.process_demux_data(&flv_demux_data)?;

        Ok(())
    }

    pub fn flush_remaining_data(&mut self) -> Result<(), MediaError> {
        let data = self.ts_muxer.get_data();

        let name = self.ts_handler.write(data)?;

        let mut discontinuity: bool = false;
        if self.last_dts > self.last_ts_dts + 15 * 1000 {
            discontinuity = true;
        }
        self.m3u8_handler.write_m3u8_header()?;
        self.m3u8_handler.add_segment(
            name,
            self.last_pts,
            self.last_dts - self.last_ts_dts,
            discontinuity,
            true,
        )?;

        Ok(())
    }

    pub fn process_demux_data(
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

                pts = data.pts;
                dts = data.dts;
                pid = self.video_pid;
                payload.extend_from_slice(&data.data[..]);

                if data.frame_type == frame_type::KEY_FRAME {
                    flags = MPEG_FLAG_IDR_FRAME;
                    if dts - self.last_ts_dts >= self.duration * 1000 {
                        self.need_new_segment = true;
                    }
                }
            }
            FlvDemuxerData::Audio { data } => {
                if !data.has_data {
                    return Ok(());
                }

                pts = data.pts;
                dts = data.dts;
                pid = self.audio_pid;
                payload.extend_from_slice(&data.data[..]);
            }
            _ => return Ok(()),
        }

        if self.need_new_segment {
            let data = self.ts_muxer.get_data();
            let name = self.ts_handler.write(data)?;

            let mut discontinuity: bool = false;
            if dts > self.last_ts_dts + 15 * 1000 {
                discontinuity = true;
            }
            self.m3u8_handler.write_m3u8_header()?;
            self.m3u8_handler.add_segment(
                name,
                pts,
                dts - self.last_ts_dts,
                discontinuity,
                false,
            )?;

            self.ts_muxer.reset();
            self.last_ts_dts = dts;
            self.last_ts_pts = pts;
            self.need_new_segment = false;
        }

        self.last_dts = dts;
        self.last_pts = pts;

        self.ts_muxer
            .write(pid, pts * 90, dts * 90, flags, payload)?;

        Ok(())
    }
}
