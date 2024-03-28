use crate::{
    flv_tag_header::{AudioTagHeader, VideoTagHeader},
    Unmarshal,
};

use {
    super::{
        define::{aac_packet_type, avc_packet_type, tag_type, AvcCodecId, FlvData, SoundFormat},
        errors::FlvDemuxerError,
        mpeg4_aac::Mpeg4AacProcessor,
        mpeg4_avc::Mpeg4AvcProcessor,
    },
    byteorder::BigEndian,
    bytes::BytesMut,
    bytesio::bytes_reader::BytesReader,
};

/*
 ** Flv Struct **
 +-------------------------------------------------------------------------------+
 | FLV header(9 bytes) | FLV body                                                |
 +-------------------------------------------------------------------------------+
 |                     | PreviousTagSize0(4 bytes)| Tag1|PreviousTagSize1|Tag2|...
 +-------------------------------------------------------------------------------+

 *** Flv Tag ***
 +-------------------------------------------------------------------------------------------------------------------------------+
 |                                                    Tag1                                                                       |
 +-------------------------------------------------------------------------------------------------------------------------------+
 |     Tag Header                                                                                                   |  Tag Data  |
 +-------------------------------------------------------------------------------------------------------------------------------+
 | Tag Type(1 byte) | Data Size(3 bytes) | Timestamp(3 bytes dts) | Timestamp Extended(1 byte) | Stream ID(3 bytes) |  Tag Data  |
 +-------------------------------------------------------------------------------------------------------------------------------+


  The Tag Data contains
  - video tag data
  - audio tag data

 **** Video Tag ****
 +-------------------------------------------------+
 |    Tag Data  (Video Tag)                        |
 +-------------------------------------------------+
 | FrameType(4 bits) | CodecID(4 bits) | Video Data|
 +-------------------------------------------------+

  The contents of Video Data depends on the codecID:
  2: H263VIDEOPACKET
  3: SCREENVIDEOPACKET
  4: VP6FLVVIDEOPACKET
  5: VP6FLVALPHAVIDEOPACKET
  6: SCREENV2VIDEOPACKET
  7: AVCVIDEOPACKE

 When the codecid equals 7, the Video Data's struct is as follows:

 +------------------------------------------------------------+
 |    Video Data  (codecID == 7)                              |
 +------------------------------------------------------------+
 | AVCPacketType(1 byte) | CompositionTime(3 bytes) | Payload |
 +------------------------------------------------------------+

 **** Audio Tag ****
 +----------------------------------------------------------------------------------------+
 |    Tag Data  (Audio Tag)                                                               |
 +----------------------------------------------------------------------------------------+
 | SoundFormat(4 bits) | SoundRate(2 bits) | SoundSize(1 bit) | SoundType(1 bit)| Payload |
 +----------------------------------------------------------------------------------------+

 reference: https://www.cnblogs.com/chyingp/p/flv-getting-started.html
*/

#[derive(Default)]
pub struct FlvDemuxerAudioData {
    pub has_data: bool,
    pub sound_format: u8,
    pub dts: i64,
    pub pts: i64,
    pub data: BytesMut,
}

impl FlvDemuxerAudioData {
    pub fn new() -> Self {
        Self {
            has_data: false,
            sound_format: 0,
            dts: 0,
            pts: 0,
            data: BytesMut::new(),
        }
    }
}
#[derive(Default)]
pub struct FlvDemuxerVideoData {
    pub frame_type: u8,
    pub codec_id: u8,
    pub dts: i64,
    pub pts: i64,
    pub data: BytesMut,
}

impl FlvDemuxerVideoData {
    pub fn new() -> Self {
        Self {
            codec_id: 0,
            dts: 0,
            pts: 0,
            frame_type: 0,
            data: BytesMut::new(),
        }
    }
}

#[derive(Default)]
pub struct FlvVideoTagDemuxer {
    avc_processor: Mpeg4AvcProcessor,
}

impl FlvVideoTagDemuxer {
    pub fn new() -> Self {
        Self {
            avc_processor: Mpeg4AvcProcessor::new(),
        }
    }
    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<Option<FlvDemuxerVideoData>, FlvDemuxerError> {
        let mut reader = BytesReader::new(data);

        let tag_header = VideoTagHeader::unmarshal(&mut reader)?;
        if tag_header.codec_id == AvcCodecId::H264 as u8 {
            match tag_header.avc_packet_type {
                avc_packet_type::AVC_SEQHDR => {
                    self.avc_processor
                        .decoder_configuration_record_load(&mut reader)?;

                    return Ok(None);
                }
                avc_packet_type::AVC_NALU => {
                    let data = self.avc_processor.h264_mp4toannexb(&mut reader)?;

                    let video_data = FlvDemuxerVideoData {
                        codec_id: AvcCodecId::H264 as u8,
                        pts: timestamp as i64 + tag_header.composition_time as i64,
                        dts: timestamp as i64,
                        frame_type: tag_header.frame_type,
                        data,
                    };
                    //print!("flv demux video payload length {}\n", video_data.data.len());
                    return Ok(Some(video_data));
                }
                _ => {}
            }
        }

        Ok(None)
    }
}

#[derive(Default)]
pub struct FlvAudioTagDemuxer {
    aac_processor: Mpeg4AacProcessor,
}

impl FlvAudioTagDemuxer {
    pub fn new() -> Self {
        Self {
            aac_processor: Mpeg4AacProcessor::new(),
        }
    }

    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<FlvDemuxerAudioData, FlvDemuxerError> {
        let mut reader = BytesReader::new(data);

        let tag_header = AudioTagHeader::unmarshal(&mut reader)?;
        self.aac_processor
            .extend_data(reader.extract_remaining_bytes());

        if tag_header.sound_format == SoundFormat::AAC as u8 {
            match tag_header.aac_packet_type {
                aac_packet_type::AAC_SEQHDR => {
                    if self.aac_processor.bytes_reader.len() >= 2 {
                        self.aac_processor.audio_specific_config_load()?;
                    }

                    return Ok(FlvDemuxerAudioData::new());
                }
                aac_packet_type::AAC_RAW => {
                    self.aac_processor.adts_save()?;

                    let audio_data = FlvDemuxerAudioData {
                        has_data: true,
                        sound_format: tag_header.sound_format,
                        pts: timestamp as i64,
                        dts: timestamp as i64,
                        data: self.aac_processor.bytes_writer.extract_current_bytes(),
                    };
                    //print!("flv demux audio payload length {}\n", audio_data.data.len());
                    return Ok(audio_data);
                }
                _ => {}
            }
        }

        Ok(FlvDemuxerAudioData::new())
    }
}

pub struct FlvDemuxer {
    bytes_reader: BytesReader,
}

impl FlvDemuxer {
    pub fn new(data: BytesMut) -> Self {
        Self {
            bytes_reader: BytesReader::new(data),
        }
    }

    pub fn read_flv_header(&mut self) -> Result<(), FlvDemuxerError> {
        /*flv header*/
        self.bytes_reader.read_bytes(9)?;
        Ok(())
    }

    pub fn read_flv_tag(&mut self) -> Result<Option<FlvData>, FlvDemuxerError> {
        /*previous_tag_size*/
        self.bytes_reader.read_u32::<BigEndian>()?;

        /*tag type*/
        let tag_type = self.bytes_reader.read_u8()?;
        /*data size*/
        let data_size = self.bytes_reader.read_u24::<BigEndian>()?;
        /*timestamp*/
        let timestamp = self.bytes_reader.read_u24::<BigEndian>()?;
        /*timestamp extended*/
        let timestamp_ext = self.bytes_reader.read_u8()?;
        /*stream id*/
        self.bytes_reader.read_u24::<BigEndian>()?;

        let dts: u32 = (timestamp & 0xffffff) | ((timestamp_ext as u32) << 24);

        /*data*/
        let body = self.bytes_reader.read_bytes(data_size as usize)?;

        match tag_type {
            tag_type::VIDEO => {
                return Ok(Some(FlvData::Video {
                    timestamp: dts,
                    data: body,
                }));
            }
            tag_type::AUDIO => {
                return Ok(Some(FlvData::Audio {
                    timestamp: dts,
                    data: body,
                }));
            }

            _ => {}
        }

        Ok(None)
    }
}
