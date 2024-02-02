use crate::stream::StreamIdentifier;

use {
    super::StreamStatistics,
    std::{sync::Arc, time::Duration},
    tokio::{
        sync::{
            mpsc,
            mpsc::{Receiver, Sender},
            Mutex,
        },
        time,
    },
    xflv::{
        define,
        define::{aac_packet_type, AvcCodecId, SoundFormat},
        mpeg4_aac::Mpeg4Aac,
        mpeg4_avc::Mpeg4Avc,
    },
};

pub struct AvStatistics {
    /*used to calculate video bitrate */
    video_bytes: Arc<Mutex<f32>>,
    /*used to calculate audio bitrate */
    audio_bytes: Arc<Mutex<f32>>,
    //used to calculate frame rate
    frame_count: Arc<Mutex<usize>>,
    //used to calculate GOP
    gop_frame_count: Arc<Mutex<usize>>,
    stream_statistics: Arc<Mutex<StreamStatistics>>,
    pub sender: Sender<bool>,
}

impl AvStatistics {
    pub fn new(identifier: StreamIdentifier) -> Self {
        let (s, _): (Sender<bool>, Receiver<bool>) = mpsc::channel(1);
        Self {
            video_bytes: Arc::new(Mutex::new(0.0)),
            audio_bytes: Arc::new(Mutex::new(0.0)),
            frame_count: Arc::new(Mutex::new(0)),
            gop_frame_count: Arc::new(Mutex::new(0)),
            stream_statistics: Arc::new(Mutex::new(StreamStatistics::new(identifier))),
            sender: s,
        }
    }

    pub async fn notify_audio_codec_info(&mut self, codec_info: &Mpeg4Aac) {
        let audio_info = &mut self.stream_statistics.lock().await.audio;
        audio_info.profile = define::u8_2_aac_profile(codec_info.object_type);
        audio_info.samplerate = codec_info.sampling_frequency;
        audio_info.sound_format = SoundFormat::AAC;
        audio_info.channels = codec_info.channels;
    }

    pub async fn notify_video_codec_info(&mut self, codec_info: &Mpeg4Avc) {
        let video_info = &mut self.stream_statistics.lock().await.video;
        video_info.codec = AvcCodecId::H264;
        video_info.profile = define::u8_2_avc_profile(codec_info.profile);
        video_info.level = define::u8_2_avc_level(codec_info.level);
        video_info.height = codec_info.height;
        video_info.width = codec_info.width;
    }

    pub async fn notify_audio_statistics_info(&mut self, data_size: usize, aac_packet_type: u8) {
        match aac_packet_type {
            aac_packet_type::AAC_RAW => {
                *self.audio_bytes.lock().await += data_size as f32;
            }
            aac_packet_type::AAC_SEQHDR => {}
            _ => {}
        }
    }

    pub async fn notify_video_statistics_info(&mut self, data_size: usize, is_key_frame: bool) {
        *self.video_bytes.lock().await += data_size as f32;
        *self.frame_count.lock().await += 1;
        if is_key_frame {
            let video_info = &mut self.stream_statistics.lock().await.video;
            video_info.gop = *self.gop_frame_count.lock().await;
            *self.gop_frame_count.lock().await = 0;
        } else {
            *self.gop_frame_count.lock().await += 1;
        }
    }

    pub fn start(&mut self) {
        let mut interval = time::interval(Duration::from_secs(1));

        let video_bytes_clone = self.video_bytes.clone();
        let audio_bytes_clone = self.audio_bytes.clone();
        let frame_count_clone = self.frame_count.clone();
        let stream_statistics_clone = self.stream_statistics.clone();

        let (s, mut r): (Sender<bool>, Receiver<bool>) = mpsc::channel(1);
        self.sender = s;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                   _ = interval.tick() => {
                    {
                        let stream_statistics = &mut stream_statistics_clone.lock().await;
                        let audio_info = &mut stream_statistics.audio;
                        audio_info.bitrate = *audio_bytes_clone.lock().await * 8.0/1000.0;

                        let video_info = &mut stream_statistics.video;
                        video_info.bitrate = *video_bytes_clone.lock().await * 8.0/1000.0;
                        video_info.frame_rate = *frame_count_clone.lock().await;
                    }
                    *video_bytes_clone.lock().await = 0.0;
                    *audio_bytes_clone.lock().await = 0.0;
                    *frame_count_clone.lock().await = 0;
                    // if let Ok(strinfo) = serde_json::to_string(&*stream_statistics_clone.lock().await) {
                    //    // log::info!("stream_info: {strinfo}");
                    // }
                },
                   _ = r.recv() =>{
                        log::info!("avstatistics shutting down");
                        return
                   },
                }
            }
        });
    }

    pub async fn get_avstatistic_data(&self) -> StreamStatistics {
        self.stream_statistics.lock().await.clone()
    }
}
