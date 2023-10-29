use {
    super::{define::FlvDemuxerData, errors::MediaError, m3u8::M3u8},
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
}

impl Flv2HlsRemuxer {
    pub fn new(duration: i64, app_name: String, stream_name: String, need_record: bool) -> Self {
        let mut ts_muxer = TsMuxer::new();
        let audio_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_AAC)
            .unwrap();
        let video_pid = ts_muxer
            .add_stream(epsi_stream_type::PSI_STREAM_H264)
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

            m3u8_handler: M3u8::new(duration, 6, app_name, stream_name, need_record),
        }
    }

    pub fn process_flv_data(&mut self, data: FlvData) -> Result<(), MediaError> {
        let flv_demux_data: FlvDemuxerData = match data {
            FlvData::Audio { timestamp, data } => {
                let audio_data = self.audio_demuxer.demux(timestamp, data)?;
                FlvDemuxerData::Audio { data: audio_data }
            }
            FlvData::Video { timestamp, data } => {
                if let Some(video_data) = self.video_demuxer.demux(timestamp, data)? {
                    FlvDemuxerData::Video { data: video_data }
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        };

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
        self.m3u8_handler.refresh_playlist()?;

        Ok(())
    }

    pub fn process_demux_data(
        &mut self,
        flv_demux_data: &FlvDemuxerData,
    ) -> Result<(), MediaError> {
        self.need_new_segment = false;

        let pid: u16;
        let pts: i64;
        let dts: i64;
        let mut flags: u16 = 0;
        let mut payload: BytesMut = BytesMut::new();

        match flv_demux_data {
            FlvDemuxerData::Video { data } => {
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
            let mut discontinuity: bool = false;
            if dts > self.last_ts_dts + 15 * 1000 {
                discontinuity = true;
            }
            let data = self.ts_muxer.get_data();

            self.m3u8_handler
                .add_segment(dts - self.last_ts_dts, discontinuity, false, data)?;
            self.m3u8_handler.refresh_playlist()?;

            self.ts_muxer.reset();
            self.last_ts_dts = dts;
            self.last_ts_pts = pts;
            self.need_new_segment = false;
        }

        self.last_dts = dts;
        self.last_pts = pts;

        self.ts_muxer
            .write(pid, pts as u64 * 90, dts as u64 * 90, flags, payload)?;

        Ok(())
    }

    pub fn clear_files(&mut self) -> Result<(), MediaError> {
        self.m3u8_handler.clear()
    }
}
#[cfg(test)]
mod tests {
    // use std::{
    //     env,
    //     fs::{self},
    // };

    // #[test]
    // fn test_new_path() {
    //     if let Ok(current_dir) = env::current_dir() {
    //         println!("Current directory: {:?}", current_dir);
    //     } else {
    //         eprintln!("Failed to get the current directory");
    //     }
    //     let directory = "test";

    //     if !fs::metadata(directory).is_ok() {
    //         match fs::create_dir(directory) {
    //             Ok(_) => println!("目录已创建"),
    //             Err(err) => println!("创建目录时出错：{:?}", err),
    //         }
    //     } else {
    //         println!("目录已存在");
    //     }
    // }
    // #[test]
    // fn test_copy() {
    //     let path = "./aa.txt";
    //     if let Err(err) = fs::copy(path, "./test/") {
    //         println!("copy err: {err}");
    //     } else {
    //         println!("copy success");
    //     }
    // }
}
