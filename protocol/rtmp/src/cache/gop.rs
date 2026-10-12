use {std::collections::VecDeque, streamhub::define::FrameData};

// Default upper bound on the number of frames a single GOP may hold.
// The GOP cache normally rotates when a video key frame arrives. Audio-only
// streams never produce a key frame, so without this cap a single GOP would
// accumulate every frame indefinitely and leak memory until OOM (issue #190).
// A realistic video GOP rotates on its key frame long before reaching the cap,
// so it only affects pathological streams. Overridable via gop_max_frame_num.
//
// This is a per-GOP cap, so the whole cache is bounded by gop_num * this value.
// For audio-only streams it also bounds how much history a new subscriber gets
// on join: at ~23 ms per AAC frame, 2000 frames is roughly 46 s. Lower it via
// gop_max_frame_num if a low-latency audio start matters more than buffered
// history.
const DEFAULT_MAX_GOP_FRAME_NUM: usize = 2000;

#[derive(Clone)]
pub struct Gop {
    datas: Vec<FrameData>,
}

impl Default for Gop {
    fn default() -> Self {
        Self::new()
    }
}

impl Gop {
    pub fn new() -> Self {
        Self { datas: Vec::new() }
    }

    fn save_frame_data(&mut self, data: FrameData) {
        self.datas.push(data);
    }

    pub fn get_frame_data(self) -> Vec<FrameData> {
        self.datas
    }

    pub fn len(&self) -> usize {
        self.datas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone)]
pub struct Gops {
    gops: VecDeque<Gop>,
    size: usize,
    // Maximum number of frames allowed in a single GOP before it is rotated,
    // even when no video key frame arrives. Bounds memory for audio-only
    // streams (see DEFAULT_MAX_GOP_FRAME_NUM).
    max_frame_num: usize,
}

impl Default for Gops {
    fn default() -> Self {
        Self::new(1, None)
    }
}

impl Gops {
    // Creates a GOP cache holding at most `size` GOPs, each capped at
    // `max_frame_num` frames. A None (config unset) or 0 cap falls back to the
    // default; 0 would otherwise rotate on every frame and defeat the cache.
    pub fn new(size: usize, max_frame_num: Option<usize>) -> Self {
        Self {
            gops: VecDeque::from([Gop::new()]),
            size,
            max_frame_num: max_frame_num
                .filter(|&num| num > 0)
                .unwrap_or(DEFAULT_MAX_GOP_FRAME_NUM),
        }
    }

    pub fn save_frame_data(&mut self, data: FrameData, is_key_frame: bool) {
        if self.size == 0 {
            return;
        }

        // A video key frame starts a new GOP. For audio-only streams no key
        // frame ever arrives, so we also rotate once the current GOP reaches
        // its frame cap; otherwise it would grow without bound (issue #190).
        //
        // The cap only rotates on an audio frame. Video keeps rotating strictly
        // on key frames, so a video GOP never gets split on a non-key frame:
        // that would leave a new subscriber with a GOP that has no leading key
        // frame and thus nothing decodable. This matters when gop_max_frame_num
        // is configured smaller than the video key-frame interval.
        let audio_gop_is_full = matches!(data, FrameData::Audio { .. })
            && self
                .gops
                .back()
                .is_some_and(|gop| gop.len() >= self.max_frame_num);

        if is_key_frame || audio_gop_is_full {
            //todo It may be possible to optimize here
            if self.gops.len() == self.size {
                self.gops.pop_front();
            }
            self.gops.push_back(Gop::new());
        }

        if let Some(gop) = self.gops.back_mut() {
            gop.save_frame_data(data);
        } else {
            log::error!("should not be here!");
        }
    }

    pub fn setted(&self) -> bool {
        self.size != 0
    }

    pub fn get_gops(&self) -> VecDeque<Gop> {
        self.gops.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn audio_frame() -> FrameData {
        FrameData::Audio {
            timestamp: 0,
            data: BytesMut::new(),
        }
    }

    // Key-frame-ness is carried by the `is_key_frame` flag passed to
    // save_frame_data, not by the payload, so a key frame and an inter frame
    // build the same FrameData::Video and differ only in that flag.
    fn video_key_frame() -> FrameData {
        FrameData::Video {
            timestamp: 0,
            data: BytesMut::new(),
        }
    }

    fn video_inter_frame() -> FrameData {
        FrameData::Video {
            timestamp: 0,
            data: BytesMut::new(),
        }
    }

    fn total_frames(gops: &Gops) -> usize {
        gops.get_gops().iter().map(|gop| gop.len()).sum()
    }

    // Regression test for issue #190: an audio-only stream never produces a
    // video key frame, so without a frame-count cap the single GOP grows
    // without bound and eventually OOMs the process. Feed far more frames than
    // the cap and assert the cache stays bounded.
    #[test]
    fn audio_only_stream_is_bounded() {
        let mut gops = Gops::new(1, None);
        for _ in 0..(DEFAULT_MAX_GOP_FRAME_NUM * 5) {
            gops.save_frame_data(audio_frame(), false);
        }
        assert!(total_frames(&gops) <= DEFAULT_MAX_GOP_FRAME_NUM);
    }

    // The cap is configurable: a smaller value bounds the cache more tightly.
    #[test]
    fn configured_cap_is_respected() {
        let cap = 10;
        let mut gops = Gops::new(1, Some(cap));
        for _ in 0..(cap * 10) {
            gops.save_frame_data(audio_frame(), false);
        }
        assert!(total_frames(&gops) <= cap);
    }

    // A cap of 0 is nonsensical (it would rotate on every frame), so it falls
    // back to the default rather than disabling the cache.
    #[test]
    fn zero_cap_falls_back_to_default() {
        let mut gops = Gops::new(1, Some(0));
        for _ in 0..(DEFAULT_MAX_GOP_FRAME_NUM * 2) {
            gops.save_frame_data(audio_frame(), false);
        }
        let frames = total_frames(&gops);
        assert!(frames > 1 && frames <= DEFAULT_MAX_GOP_FRAME_NUM);
    }

    // Normal video behavior is unchanged: each key frame opens a new GOP and
    // the number of retained GOPs never exceeds the configured size.
    #[test]
    fn key_frame_rotation_bounds_gop_count() {
        let size = 3;
        let mut gops = Gops::new(size, None);
        for _ in 0..10 {
            gops.save_frame_data(video_key_frame(), true);
            for _ in 0..5 {
                gops.save_frame_data(audio_frame(), false);
            }
        }
        assert!(gops.get_gops().len() <= size);
    }

    // The frame cap must never split a video GOP on a non-key frame, even when
    // gop_max_frame_num is misconfigured below the key-frame interval: doing so
    // would leave a new subscriber with a GOP that has no leading key frame and
    // nothing decodable. Video rotates strictly on key frames, so a long run of
    // inter frames past the cap stays in the single key-frame-headed GOP.
    #[test]
    fn video_gop_is_not_split_by_frame_cap() {
        let cap = 10;
        let mut gops = Gops::new(1, Some(cap));
        gops.save_frame_data(video_key_frame(), true);
        for _ in 0..(cap * 5) {
            gops.save_frame_data(video_inter_frame(), false);
        }
        assert_eq!(gops.get_gops().len(), 1);
        assert_eq!(total_frames(&gops), cap * 5 + 1);
    }

    // gop_num = 0 disables caching entirely.
    #[test]
    fn disabled_cache_stores_nothing() {
        let mut gops = Gops::new(0, None);
        gops.save_frame_data(audio_frame(), false);
        gops.save_frame_data(video_key_frame(), true);
        assert_eq!(total_frames(&gops), 0);
    }
}
