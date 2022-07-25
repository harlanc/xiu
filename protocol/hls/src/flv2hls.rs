use crate::hls_event_manager::M3u8Consumer;

use {
    super::{
        define::FlvDemuxerData, errors::MediaError, hls_event_manager::HlsEventProducer, m3u8::M3u8,
    },
    bytes::BytesMut,
    xflv::{
        define::{frame_type, FlvData},
        demuxer::{FlvAudioTagDemuxer, FlvVideoTagDemuxer},
    },
    xmpegts::{
        define::{epsi_stream_type, MPEG_FLAG_IDR_FRAME},
        ts::TsMuxer,
    },
};

pub struct Flv2HlsRemuxer {
    video_demuxer: FlvVideoTagDemuxer,
    audio_demuxer: FlvAudioTagDemuxer,

    partial_ts_muxer: TsMuxer,
    ts_muxer: TsMuxer,

    last_ts_dts: i64,
    last_partial_ts_dts: i64,
    last_ts_pts: i64,

    last_dts: i64,
    last_pts: i64,

    duration: i64,
    partial_seg_duration: i64,
    need_new_segment: bool,
    need_new_partial_segment: bool,
    segment_has_idr: bool,

    video_pid: u16,
    audio_pid: u16,

    p_video_pid: u16,
    p_audio_pid: u16,

    m3u8_handler: M3u8,
}

impl Flv2HlsRemuxer {
    pub fn new(
        hls_event_tx: HlsEventProducer,
        m3u8_consumer: M3u8Consumer,
        duration: i64,
        partial_seg_duration: i64,
        app_name: String,
        stream_name: String,
    ) -> Self {
        let mut ts_muxer = TsMuxer::new();
        let audio_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_AAC, BytesMut::new())
            .unwrap();
        let video_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_H264, BytesMut::new())
            .unwrap();

        let mut partial_ts_muxer = TsMuxer::new();
        let p_audio_pid = partial_ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_AAC, BytesMut::new())
            .unwrap();
        let p_video_pid = partial_ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_H264, BytesMut::new())
            .unwrap();

        let m3u8_name = format!("{}.m3u8", stream_name);

        let m3u8_handler = M3u8::new(
            hls_event_tx,
            duration,
            6,
            m3u8_name,
            app_name.clone(),
            stream_name.clone(),
        );

        m3u8_handler.setup_m3u8_listener(m3u8_consumer);

        Self {
            video_demuxer: FlvVideoTagDemuxer::new(),
            audio_demuxer: FlvAudioTagDemuxer::new(),

            ts_muxer,
            partial_ts_muxer,

            last_ts_dts: 0,
            last_partial_ts_dts: 0,
            last_ts_pts: 0,

            last_dts: 0,
            last_pts: 0,

            duration,
            partial_seg_duration,
            need_new_segment: false,
            need_new_partial_segment: false,
            segment_has_idr: false,

            video_pid,
            audio_pid,

            p_video_pid,
            p_audio_pid,

            m3u8_handler: m3u8_handler,
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
            _ => return Ok(()),
        }

        self.process_demux_data(&flv_demux_data)?;

        Ok(())
    }

    pub fn flush_remaining_data(&mut self) -> Result<(), MediaError> {
        let data = self.ts_muxer.get_data();
        let mut discontinuity: bool = false;
        if self.last_dts > self.last_ts_dts + 15 * 1000 {
            discontinuity = true;
        }
        self.m3u8_handler.add_segment(
            self.last_dts - self.last_ts_dts,
            discontinuity,
            true,
            data,
        )?;
        self.m3u8_handler.refresh_playlist(false)?;

        Ok(())
    }

    pub fn process_demux_data(
        &mut self,
        flv_demux_data: &FlvDemuxerData,
    ) -> Result<(), MediaError> {
        self.need_new_segment = false;
        self.need_new_partial_segment = false;

        let pid: u16;
        let p_pid: u16;
        let pts: i64;
        let dts: i64;
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
                p_pid = self.video_pid;

                payload.extend_from_slice(&data.data[..]);

                if data.frame_type == frame_type::KEY_FRAME {
                    flags = MPEG_FLAG_IDR_FRAME;
                    self.segment_has_idr = true;
                }

                if dts - self.last_ts_dts >= self.duration * 1000 {
                    self.need_new_segment = true;
                } else if dts - self.last_partial_ts_dts >= self.partial_seg_duration {
                    self.need_new_partial_segment = true;
                }
            }
            FlvDemuxerData::Audio { data } => {
                if !data.has_data {
                    return Ok(());
                }

                pts = data.pts;
                dts = data.dts;
                pid = self.audio_pid;
                p_pid = self.audio_pid;
                payload.extend_from_slice(&data.data[..]);
            }
            _ => return Ok(()),
        }

        if self.need_new_partial_segment {
            let d = self.partial_ts_muxer.get_data();

            println!(" f: {}", flags);

            self.m3u8_handler.add_partial_segment(
                dts - self.last_partial_ts_dts,
                d,
                self.segment_has_idr,
            )?;
            self.m3u8_handler.refresh_playlist(false)?;

            self.partial_ts_muxer.reset();
            self.last_partial_ts_dts = dts;
            self.need_new_partial_segment = false;
            self.segment_has_idr = false;
        }

        if self.need_new_segment {
            let mut discontinuity: bool = false;
            if dts > self.last_ts_dts + 15 * 1000 {
                discontinuity = true;
            }
            let data = self.ts_muxer.get_data();

            self.m3u8_handler
                .add_segment(dts - self.last_ts_dts, discontinuity, false, data)?;
            self.m3u8_handler.refresh_playlist(true)?;

            self.ts_muxer.reset();
            self.last_ts_dts = dts;
            self.last_ts_pts = pts;
            self.need_new_segment = false;
            self.segment_has_idr = false;
        }

        self.last_dts = dts;
        self.last_pts = pts;

        self.ts_muxer
            .write(pid, pts * 90, dts * 90, flags, payload.clone())?;

        self.partial_ts_muxer
            .write(p_pid, pts * 90, dts * 90, flags, payload)?;

        Ok(())
    }

    pub fn clear_files(&mut self) -> Result<(), MediaError> {
        self.m3u8_handler.clear()
    }
}
