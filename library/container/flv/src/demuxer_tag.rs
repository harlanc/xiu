use {
    super::{define, errors::FlvDemuxerError},
    bytes::BytesMut,
    bytesio::bytes_reader::BytesReader,
};

#[derive(Clone)]
pub struct AudioTagHeader {
    /*
        SoundFormat: UB[4]
        0 = Linear PCM, platform endian
        1 = ADPCM
        2 = MP3
        3 = Linear PCM, little endian
        4 = Nellymoser 16-kHz mono
        5 = Nellymoser 8-kHz mono
        6 = Nellymoser
        7 = G.711 A-law logarithmic PCM
        8 = G.711 mu-law logarithmic PCM
        9 = reserved
        10 = AAC
        11 = Speex
        14 = MP3 8-Khz
        15 = Device-specific sound
        Formats 7, 8, 14, and 15 are reserved for internal use
        AAC is supported in Flash Player 9,0,115,0 and higher.
        Speex is supported in Flash Player 10 and higher.
    */
    pub sound_format: u8,
    /*
        SoundRate: UB[2]
        Sampling rate
        0 = 5.5-kHz For AAC: always 3
        1 = 11-kHz
        2 = 22-kHz
        3 = 44-kHz
    */
    pub sound_rate: u8,
    /*
        SoundSize: UB[1]
        0 = snd8Bit
        1 = snd16Bit
        Size of each sample.
        This parameter only pertains to uncompressed formats.
        Compressed formats always decode to 16 bits internally
    */
    pub sound_size: u8,
    /*
        SoundType: UB[1]
        0 = sndMono
        1 = sndStereo
        Mono or stereo sound For Nellymoser: always 0
        For AAC: always 1
    */
    pub sound_type: u8,

    /*
        0: AAC sequence header
        1: AAC raw
    */
    pub aac_packet_type: u8,
}

impl AudioTagHeader {
    pub fn defalut() -> Self {
        AudioTagHeader {
            sound_format: 0,
            sound_rate: 0,
            sound_size: 0,
            sound_type: 0,
            aac_packet_type: 0,
        }
    }
}

pub struct AudioTagHeaderDemuxer {
    bytes_reader: BytesReader,
    tag: AudioTagHeader,
}

impl AudioTagHeaderDemuxer {
    pub fn new(data: BytesMut) -> Self {
        Self {
            bytes_reader: BytesReader::new(data),
            tag: AudioTagHeader::defalut(),
        }
    }

    pub fn parse_tag_header(&mut self) -> Result<AudioTagHeader, FlvDemuxerError> {
        let flags = self.bytes_reader.read_u8()?;

        self.tag.sound_format = flags >> 4;
        self.tag.sound_rate = (flags >> 2) & 0x03;
        self.tag.sound_size = (flags >> 1) & 0x01;
        self.tag.sound_type = flags & 0x01;

        match self.tag.sound_format {
            define::sound_format::AAC => {
                self.tag.aac_packet_type = self.bytes_reader.read_u8()?;
            }
            _ => {}
        }

        return Ok(self.tag.clone());
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        return self.bytes_reader.extract_remaining_bytes();
    }
}
#[derive(Clone)]
pub struct VideoTagHeader {
    /*
        1: keyframe (for AVC, a seekable frame)
        2: inter frame (for AVC, a non- seekable frame)
        3: disposable inter frame (H.263 only)
        4: generated keyframe (reserved for server use only)
        5: video info/command frame
    */
    pub frame_type: u8,
    /*
        1: JPEG (currently unused)
        2: Sorenson H.263
        3: Screen video
        4: On2 VP6
        5: On2 VP6 with alpha channel
        6: Screen video version 2
        7: AVC
    */
    pub codec_id: u8,
    /*
        0: AVC sequence header
        1: AVC NALU
        2: AVC end of sequence (lower level NALU sequence ender is not required or supported)
    */
    pub avc_packet_type: u8,
    pub composition_time: u32,
}

impl VideoTagHeader {
    pub fn defalut() -> Self {
        VideoTagHeader {
            frame_type: 0,
            codec_id: 0,
            avc_packet_type: 0,
            composition_time: 0,
        }
    }
}

pub struct VideoTagHeaderDemuxer {
    bytes_reader: BytesReader,
    tag: VideoTagHeader,
}

impl VideoTagHeaderDemuxer {
    pub fn new(data: BytesMut) -> Self {
        Self {
            bytes_reader: BytesReader::new(data),
            tag: VideoTagHeader::defalut(),
        }
    }

    pub fn parse_tag_header(&mut self) -> Result<VideoTagHeader, FlvDemuxerError> {
        let flags = self.bytes_reader.read_u8()?;

        self.tag.frame_type = flags >> 4;
        self.tag.codec_id = flags & 0x0f;

        if self.tag.codec_id == define::codec_id::FLV_VIDEO_H264
            || self.tag.codec_id == define::codec_id::FLV_VIDEO_H265
        {
            self.tag.avc_packet_type = self.bytes_reader.read_u8()?;
            self.tag.composition_time = 0;

            //bigend 3bytes
            for _ in 0..3 {
                let time = self.bytes_reader.read_u8()?;
                //print!("==time0=={}\n", time);
                //print!("==time1=={}\n", self.tag.composition_time);
                self.tag.composition_time = (self.tag.composition_time << 8) + time as u32;
            }
        }

        return Ok(self.tag.clone());
    }

    pub fn get_remaining_bytes(&mut self) -> BytesMut {
        return self.bytes_reader.extract_remaining_bytes();
    }
}
