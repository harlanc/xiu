use std::{collections::VecDeque, fmt::format};

use rtmp::messages::define::msg_type_id;

use std::{fs::File, io::Write};

pub struct Segment {
    pts: i64,
    /*ts duration*/
    duration: i64,
    discontinuity: bool,
    /*ts name*/
    name: String,
}

impl Segment {
    pub fn new(pts: i64, duration: i64, discontinuity: bool, name: String) -> Self {
        Self {
            pts,
            duration,
            discontinuity,
            name,
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
    m3u8_header: String,

    m3u8_file_handler: File,
}

impl M3u8 {
    pub fn new(duration: i64, live_ts_count: usize, name: String) -> Self {
        let file_handler = File::create(name).unwrap();

        Self {
            version: 3,
            sequence_no: 0,
            duration,
            is_live: true,
            live_ts_count,
            segments: VecDeque::new(),
            m3u8_header: String::new(),
            m3u8_file_handler: file_handler,
        }
    }
    pub fn flush(&mut self) {}
    pub fn add_segment(&mut self, name: String, pts: i64, duration: i64, discontinuity: bool) {
        let segment_count = self.segments.len();

        if self.is_live && segment_count >= self.live_ts_count {
            self.segments.pop_front();
        }

        self.duration = std::cmp::max(duration, self.duration);
        self.sequence_no += 1;

        let segment = Segment::new(pts, duration, discontinuity, name.clone());
        self.segments.push_back(segment);

        /*flush to file*/
        let mut segment_content: String = String::from("");
        if discontinuity {
            segment_content += "#EXT-X-DISCONTINUITY\n";
        }
        segment_content += format!("#EXTINF:{:.3}\n{}\n", duration as f64 / 1000.0, name).as_str();
        self.m3u8_file_handler.write(segment_content.as_bytes());
    }

    fn init_m3u8_header(&mut self) {
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

        /*flush to file*/
        self.m3u8_file_handler.write(self.m3u8_header.as_bytes());
    }

    pub fn generate_m3u8_content(&mut self) -> String {
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
        }

        return m3u8_content;
    }
}
