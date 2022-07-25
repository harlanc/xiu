use std::{
    ops::Add,
    sync::{Arc, RwLock},
};

use crate::hls_event_manager::{HlsEventConsumer, M3u8Consumer, M3u8Event};

use {
    super::{
        errors::MediaError,
        hls_event_manager::{HlsEvent, HlsEventProducer},
        ts::Ts,
    },
    bytes::BytesMut,
    std::{collections::VecDeque, fs, fs::File, io::Write},
};

#[derive(Clone)]
pub struct PartialSegment {
    duration: i64,
    name: String,
    independent: bool,
}

#[derive(Clone)]
pub struct Segment {
    /*ts duration*/
    duration: i64,
    discontinuity: bool,
    /*ts name*/
    name: String,
    path: String,
    is_eof: bool,
    is_complete: bool,

    // LLHLS partial segments
    partials: Vec<PartialSegment>,
}

impl Segment {
    pub fn new(
        duration: i64,
        discontinuity: bool,
        name: String,
        path: String,
        is_eof: bool,
        is_complete: bool,
    ) -> Self {
        Self {
            duration,
            discontinuity,
            name,
            path,
            is_eof,
            is_complete,
            partials: vec![],
        }
    }

    pub fn set_complete(&mut self) {
        self.is_complete = true;
    }

    pub fn add_partial(&mut self, seg: PartialSegment) {
        self.partials.push(seg);
    }
}

pub struct M3u8PlaylistResponse {
    pub sequence_no: u64,
}

pub struct M3u8 {
    hls_event_tx: HlsEventProducer,
    version: u16,
    sequence_no: Arc<RwLock<u64>>,
    /*What duration should media files be?
    A duration of 10 seconds of media per file seems to strike a reasonable balance for most broadcast content.
    http://devimages.apple.com/iphone/samples/bipbop/bipbopall.m3u8*/
    duration: i64,

    is_live: bool,
    /*How many files should be listed in the index file during a continuous, ongoing session?
    The normal recommendation is 3, but the optimum number may be larger.*/
    live_ts_count: usize,

    segments: VecDeque<Segment>,
    is_header_generated: bool,

    m3u8_header: String,
    m3u8_folder: String,
    m3u8_name: String,

    ts_handler: Ts,
}

impl M3u8 {
    pub fn new(
        hls_event_tx: HlsEventProducer,
        duration: i64,
        live_ts_count: usize,
        name: String,
        app_name: String,
        stream_name: String,
    ) -> Self {
        let m3u8_folder = format!("./{}/{}", app_name, stream_name);
        fs::create_dir_all(m3u8_folder.clone()).unwrap();

        Self {
            hls_event_tx: hls_event_tx.clone(),
            version: 6,
            sequence_no: Arc::new(RwLock::new(0)),
            duration,
            is_live: true,
            live_ts_count,
            segments: VecDeque::new(),
            is_header_generated: false,
            m3u8_folder,
            m3u8_header: String::new(),
            m3u8_name: name,
            ts_handler: Ts::new(app_name, stream_name),
        }
    }

    pub fn setup_m3u8_listener(&self, mut m3u8_consumer: M3u8Consumer) {
        let seq = Arc::clone(&self.sequence_no);

        tokio::spawn(async move {
            while let Some(cmd) = m3u8_consumer.recv().await {
                use M3u8Event::*;
                match cmd {
                    RequestPlaylist { channel: c } => {
                        c.send(M3u8PlaylistResponse {
                            sequence_no: *seq.read().unwrap(),
                        })
                        .unwrap_or_default();
                    }
                }
            }
        });
    }

    pub fn add_segment(
        &mut self,
        duration: i64,
        discontinuity: bool,
        is_eof: bool,
        ts_data: BytesMut,
    ) -> Result<(), MediaError> {
        let segment_count = self.segments.len();

        if self.is_live && segment_count >= self.live_ts_count {
            let segment = self.segments.pop_front().unwrap();
            // self.ts_handler.delete(segment.path);
        }

        let mut s = self.sequence_no.write().unwrap();
        *s += 1;

        self.duration = std::cmp::max(duration, self.duration);

        self.ts_handler.write(ts_data, false)?;
        // let segment = Segment::new(duration, discontinuity, ts_name, ts_path, is_eof, false);
        self.segments.back_mut().unwrap().set_complete();

        // self.segments.push_back(segment);

        Ok(())
    }

    pub fn add_partial_segment(
        &mut self,
        duration: i64,
        ts_data: BytesMut,
        independent: bool,
    ) -> Result<(), MediaError> {
        let (ts_name, ts_path) = self.ts_handler.write(ts_data, true)?;

        let cur_seg = self.segments.back_mut();

        match cur_seg {
            None
            | Some(Segment {
                is_complete: true, ..
            }) => {
                // needs new segment

                let mut seg = Segment::new(
                    duration,
                    false,
                    format!("{}.ts", self.sequence_no.read().unwrap()),
                    ts_path,
                    false,
                    false,
                );

                let partial = PartialSegment {
                    duration,
                    name: ts_name.to_owned(),
                    independent,
                };

                seg.add_partial(partial);

                &self.segments.push_back(seg);
            }
            Some(seg) => {
                // add partial to existing segment

                let partial = PartialSegment {
                    duration,
                    name: ts_name.to_owned(),
                    independent,
                };

                println!("partial add {}", &partial.name);

                seg.add_partial(partial);
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), MediaError> {
        //clear ts
        for segment in &self.segments {
            self.ts_handler.delete(segment.path.clone());
        }
        //clear m3u8
        let m3u8_path = format!("{}/{}", self.m3u8_folder, self.m3u8_name);
        fs::remove_file(m3u8_path)?;

        Ok(())
    }

    pub fn generate_m3u8_header(&mut self) -> Result<(), MediaError> {
        self.is_header_generated = true;

        let mut playlist_type: &str = "";
        let mut allow_cache: &str = "";
        if !self.is_live {
            playlist_type = "#EXT-X-PLAYLIST-TYPE:VOD\n";
            allow_cache = "#EXT-X-ALLOW-CACHE:YES\n";
        }

        self.m3u8_header = format!("#EXTM3U\n");
        self.m3u8_header += format!("#EXT-X-VERSION:{}\n", self.version).as_str();
        self.m3u8_header +=
            format!("#EXT-X-TARGETDURATION:{}\n", (self.duration + 999) / 1000).as_str();
        self.m3u8_header += format!(
            "#EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK={}\n",
            1.5
        )
        .as_str();
        self.m3u8_header += format!(
            "#EXT-X-MEDIA-SEQUENCE:{}\n",
            self.sequence_no.read().unwrap()
        )
        .as_str();
        self.m3u8_header += playlist_type;
        self.m3u8_header += allow_cache;

        Ok(())
    }

    pub fn refresh_playlist(&mut self, broadcast_new_msn: bool) -> Result<String, MediaError> {
        self.generate_m3u8_header()?;

        let mut m3u8_content = self.m3u8_header.clone();
        for segment in &self.segments {
            if segment.discontinuity {
                m3u8_content += "#EXT-X-DISCONTINUITY\n";
            }
            if !segment.is_complete {
                for partial in &segment.partials {
                    m3u8_content += format!(
                        "#EXT-X-PART:DURATION={:.3},URI=\"{}\"{}\n",
                        partial.duration as f64 / 1000.0,
                        partial.name,
                        if partial.independent {
                            ",INDEPENDENT=YES"
                        } else {
                            ""
                        }
                    )
                    .as_str();
                }
            } else {
                m3u8_content += format!(
                    "#EXTINF:{:.3},\n{}\n",
                    segment.duration as f64 / 1000.0,
                    segment.name
                )
                .as_str();
            }

            if segment.is_eof {
                m3u8_content += "#EXT-X-ENDLIST\n";
                break;
            }
        }

        let m3u8_path = format!("{}/{}", self.m3u8_folder, self.m3u8_name);

        let mut file_handler = File::create(m3u8_path).unwrap();
        file_handler.write(m3u8_content.as_bytes())?;

        if broadcast_new_msn {
            self.hls_event_tx
                .send(HlsEvent::HlsSequenceIncr {
                    sequence: *self.sequence_no.read().unwrap(),
                })
                .unwrap_or_default();
        }

        Ok(m3u8_content)
    }
}
