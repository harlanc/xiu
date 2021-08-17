use {
    super::{errors::MediaError, ts::Ts},
    bytes::BytesMut,
    std::{collections::VecDeque, fs, fs::File, io::Write},
};

pub struct Segment {
    /*ts duration*/
    duration: i64,
    discontinuity: bool,
    /*ts name*/
    name: String,
    path: String,
    is_eof: bool,
}

impl Segment {
    pub fn new(
        duration: i64,
        discontinuity: bool,
        name: String,
        path: String,
        is_eof: bool,
    ) -> Self {
        Self {
            duration,
            discontinuity,
            name,
            path,
            is_eof,
        }
    }
}

pub struct M3u8 {
    version: u16,
    sequence_no: u64,
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
        duration: i64,
        live_ts_count: usize,
        name: String,
        app_name: String,
        stream_name: String,
    ) -> Self {
        let m3u8_folder = format!("./{}/{}", app_name, stream_name);
        fs::create_dir_all(m3u8_folder.clone()).unwrap();
        Self {
            version: 3,
            sequence_no: 0,
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
            self.ts_handler.delete(segment.path);
            self.sequence_no += 1;
        }

        self.duration = std::cmp::max(duration, self.duration);

        let (ts_name, ts_path) = self.ts_handler.write(ts_data)?;
        let segment = Segment::new(duration, discontinuity, ts_name, ts_path, is_eof);
        self.segments.push_back(segment);

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
        self.m3u8_header += format!("#EXT-X-MEDIA-SEQUENCE:{}\n", self.sequence_no).as_str();
        self.m3u8_header += playlist_type;
        self.m3u8_header += allow_cache;

        Ok(())
    }

    pub fn refresh_playlist(&mut self) -> Result<String, MediaError> {
        self.generate_m3u8_header()?;

        let mut m3u8_content = self.m3u8_header.clone();
        for segment in &self.segments {
            if segment.discontinuity {
                m3u8_content += "#EXT-X-DISCONTINUITY\n";
            }
            m3u8_content += format!(
                "#EXTINF:{:.3}\n{}\n",
                segment.duration as f64 / 1000.0,
                segment.name
            )
            .as_str();

            if segment.is_eof {
                m3u8_content += "#EXT-X-ENDLIST\n";
                break;
            }
        }

        let m3u8_path = format!("{}/{}", self.m3u8_folder, self.m3u8_name);

        let mut file_handler = File::create(m3u8_path).unwrap();
        file_handler.write(m3u8_content.as_bytes())?;

        Ok(m3u8_content)
    }
}
