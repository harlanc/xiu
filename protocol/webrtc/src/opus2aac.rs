use crate::errors::Opus2AacError;
use audiopus::coder::Decoder as OpusDecoder;
use audiopus::MutSignals;
use fdk_aac::enc::{Encoder as AacEncoder, EncoderParams};
pub struct Opus2AacTranscoder {
    decoder_channels_size: usize,
    decoder: OpusDecoder,
    encoder: AacEncoder,
    pcm_data: Vec<i16>,
}

impl Opus2AacTranscoder {
    pub fn new(
        decoder_sample_rate: i32,
        decoder_channels: audiopus::Channels,
        encoder_sample_rate: u32,
        encoder_channels: fdk_aac::enc::ChannelMode,
    ) -> Result<Self, Opus2AacError> {
        let decoder = OpusDecoder::new(
            audiopus::SampleRate::try_from(decoder_sample_rate)?,
            decoder_channels,
        )?;
        let encoder = AacEncoder::new(EncoderParams {
            bit_rate: fdk_aac::enc::BitRate::VbrMedium,
            transport: fdk_aac::enc::Transport::Raw,
            channels: encoder_channels,
            sample_rate: encoder_sample_rate,
        })?;

        let decoder_channels_size = match decoder_channels {
            audiopus::Channels::Stereo | audiopus::Channels::Auto => 2,
            audiopus::Channels::Mono => 1,
        };

        Ok(Opus2AacTranscoder {
            decoder_channels_size,
            decoder,
            encoder,
            pcm_data: Vec::new(),
        })
    }

    pub fn transcode(&mut self, input: &[u8]) -> Result<Vec<Vec<u8>>, Opus2AacError> {
        //https://opus-codec.org/docs/opus_api-1.1.2/group__opus__decoder.html#ga7d1111f64c36027ddcb81799df9b3fc9
        let mut pcm_output: Vec<i16> = vec![0; 1024 * 2];
        let input_packet = audiopus::packet::Packet::try_from(input)?;
        let mut_signals = MutSignals::try_from(&mut pcm_output)?;
        let pcm_output_len = self
            .decoder
            .decode(Some(input_packet), mut_signals, false)?;
        self.pcm_data
            .extend_from_slice(&pcm_output[..pcm_output_len * self.decoder_channels_size]);

        let mut aac_output: Vec<u8> = vec![0; 1024 * 2];
        let mut result = Vec::new();
        while self.pcm_data.len() >= 1024 * 2 {
            let pcm = self.pcm_data.split_off(2048);
            let encoder_info = self.encoder.encode(&self.pcm_data, &mut aac_output)?;
            self.pcm_data = pcm;
            if encoder_info.output_size > 0 {
                result.push(aac_output[..encoder_info.output_size].to_vec());
            }
        }

        Ok(result)
    }
}

// #[cfg(test)]
// mod tests {

//     use bytes::BytesMut;
//     use bytesio::bytes_writer::BytesWriter;
//     use fdk_aac::dec::Decoder as AacDecoder;
//     use fdk_aac::enc::Encoder as AacEncoder;
//     use opus::Decoder as OpusDecoder;
//     use opus::Encoder as OpusEncoder;
//     use std::fs::File;
//     use std::io::Read;
//     use std::io::Write;
//     use xflv::demuxer::{FlvAudioTagDemuxer, FlvDemuxer};
//     use xflv::flv_tag_header::AudioTagHeader;
//     use xflv::muxer::FlvMuxer;
//     use xflv::Marshal;

//     #[test]
//     fn test_flv_2_opus() {
//         //demux flv=> decode aac to pcm => encode pcm to opus => decode opus to pcm => encode pcm to aac =>mux aac to flv
//         let mut file = File::open("./xgplayer-demo-360p.flv").unwrap();

//         let mut flv_buffer = Vec::new();
//         file.read_to_end(&mut flv_buffer)
//             .expect("Failed to read file");

//         let flv_bytes = BytesMut::from(&flv_buffer[..]);

//         let mut flv_demuxer = FlvDemuxer::new(flv_bytes);
//         let mut audio_demuxer = FlvAudioTagDemuxer::new();

//         flv_demuxer.read_flv_header().unwrap();

//         let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
//         let aac_encoder = AacEncoder::new(fdk_aac::enc::EncoderParams {
//             bit_rate: fdk_aac::enc::BitRate::VbrMedium,
//             sample_rate: 48000,
//             transport: fdk_aac::enc::Transport::Raw,
//             channels: fdk_aac::enc::ChannelMode::Stereo,
//         })
//         .unwrap();

//         let mut opus_encoder =
//             OpusEncoder::new(48000, opus::Channels::Stereo, opus::Application::Voip).unwrap();
//         let mut opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo).unwrap();

//         let mut file = File::create("/Users/hailiang8/xgplayer-demo-360p-aac.flv").unwrap();
//         let mut flv_muxer = FlvMuxer::default();
//         flv_muxer.write_flv_header().unwrap();
//         flv_muxer.write_previous_tag_size(0).unwrap();

//         let mut audio_pcm_data_from_decoded_aac: Vec<i16> = Vec::new();
//         let mut audio_pcm_data_from_decoded_opus: Vec<i16> = Vec::new();

//         let mut time: u32 = 0;

//         loop {
//             match flv_demuxer.read_flv_tag() {
//                 Ok(Some(data)) => match data {
//                     xflv::define::FlvData::Audio { timestamp, data } => {
//                         println!("audio: time:{}", timestamp);
//                         let len = data.len() as u32;

//                         match audio_demuxer.demux(timestamp, data.clone()) {
//                             Ok(d) => {
//                                 //seq header audio asc
//                                 if !d.has_data {
//                                     flv_muxer.write_flv_tag_header(8, len, timestamp).unwrap();
//                                     flv_muxer.write_flv_tag_body(data).unwrap();
//                                     flv_muxer.write_previous_tag_size(len + 11).unwrap();

//                                     let data = flv_muxer.writer.extract_current_bytes();
//                                     file.write_all(&data[..]).unwrap();
//                                     continue;
//                                 }

//                                 println!("aac demux len: {}", d.data.len());
//                                 aac_decoder.fill(&d.data).unwrap();

//                                 let mut decode_frame = vec![0_i16; 1024 * 3];
//                                 match aac_decoder.decode_frame(&mut decode_frame[..]) {
//                                     Ok(()) => {
//                                         let len = aac_decoder.decoded_frame_size();

//                                         println!("aac decoder ok : {}", len);
//                                         audio_pcm_data_from_decoded_aac
//                                             .extend_from_slice(&decode_frame[..len]);

//                                         while audio_pcm_data_from_decoded_aac.len() >= 960 * 2 {
//                                             let pcm =
//                                                 audio_pcm_data_from_decoded_aac.split_off(960 * 2);

//                                             let mut encoded_opus = vec![0; 1500];

//                                             println!(
//                                                 "input len: {}",
//                                                 audio_pcm_data_from_decoded_aac.len()
//                                             );

//                                             match opus_encoder.encode(
//                                                 &audio_pcm_data_from_decoded_aac,
//                                                 &mut encoded_opus,
//                                             ) {
//                                                 Ok(pcm_size) => {
//                                                     let samples = opus_decoder
//                                                         .get_nb_samples(&encoded_opus)
//                                                         .unwrap();
//                                                     println!(
//                                                         "opus encode ok : {} {} samples:{}",
//                                                         pcm_size,
//                                                         audio_pcm_data_from_decoded_aac.len(),
//                                                         samples
//                                                     );
//                                                     audio_pcm_data_from_decoded_aac = pcm;

//                                                     let mut output = vec![0; 5670]; //960 * 2];

//                                                     match opus_decoder.decode(
//                                                         &encoded_opus[..pcm_size],
//                                                         &mut output,
//                                                         false,
//                                                     ) {
//                                                         Ok(size) => {
//                                                             println!("opus decode ok : {}", size);

//                                                             audio_pcm_data_from_decoded_opus
//                                                                 .extend_from_slice(
//                                                                     &output[..size * 2],
//                                                                 );

//                                                             while audio_pcm_data_from_decoded_opus
//                                                                 .len()
//                                                                 >= 2048
//                                                             {
//                                                                 let pcm = audio_pcm_data_from_decoded_opus.split_off(2048);

//                                                                 let mut encoded_aac: Vec<u8> =
//                                                                     vec![0; 1500];
//                                                                 match aac_encoder.encode(
//                                                                      &audio_pcm_data_from_decoded_opus,
//                                                                      &mut encoded_aac[..],
//                                                                  ) {
//                                                                      Ok(info) => {
//                                                                          println!("aac encode ok : {:?}", info);
//                                                                          audio_pcm_data_from_decoded_opus = pcm; //audio_pcm_data_from_decoded_opus[info.input_consumed..].to_vec();

//                                                                          if info.output_size > 0 {
//                                                                              let audio_tag_header = AudioTagHeader {
//                                                                                  sound_format: 10,
//                                                                                  sound_rate: 3,
//                                                                                  sound_size: 1,
//                                                                                  sound_type: 1,
//                                                                                  aac_packet_type: 1,
//                                                                              };

//                                                                              let tag_header_data =
//                                                                                  audio_tag_header.marshal().unwrap();

//                                                                              let mut writer = BytesWriter::new();
//                                                                              writer.write(&tag_header_data).unwrap();

//                                                                              let audio_data =
//                                                                                  &encoded_aac[..info.output_size];
//                                                                              writer.write(audio_data).unwrap();

//                                                                              let body = writer.extract_current_bytes();

//                                                                              let len = body.len() as u32;

//                                                                              flv_muxer
//                                                                                  .write_flv_tag_header(8, len, time)
//                                                                                  .unwrap();
//                                                                              flv_muxer.write_flv_tag_body(body).unwrap();
//                                                                              flv_muxer
//                                                                                  .write_previous_tag_size(len + 11)
//                                                                                  .unwrap();

//                                                                              time += 21;

//                                                                              let data =
//                                                                                  flv_muxer.writer.extract_current_bytes();
//                                                                              file.write_all(&data[..]).unwrap();
//                                                                          }
//                                                                          if info.input_consumed > 0 && info.output_size > 0 {
//                                                                          } else {
//                                                                              break;
//                                                                          }
//                                                                      }
//                                                                      Err(err) => {
//                                                                          println!("aac encode err : {}", err);
//                                                                      }
//                                                                  }
//                                                             }
//                                                         }
//                                                         Err(err) => {
//                                                             println!("opus decode err : {}", err);
//                                                         }
//                                                     }
//                                                 }
//                                                 Err(err) => {
//                                                     println!("opus encode err : {}", err);
//                                                 }
//                                             }
//                                         }
//                                     }
//                                     Err(err) => {
//                                         println!("decoder error: {}", err);
//                                         // return;
//                                     }
//                                 }
//                             }
//                             Err(err) => {
//                                 println!("demux error: {}", err);
//                             }
//                         }
//                     }
//                     xflv::define::FlvData::Video {
//                         timestamp: _,
//                         data: _,
//                     } => {
//                         println!("video");
//                     }
//                     xflv::define::FlvData::MetaData {
//                         timestamp: _,
//                         data: _,
//                     } => {
//                         println!("metadata");
//                     }
//                 },
//                 Err(err) => {
//                     println!("read error: {}", err);
//                     // file.w
//                     return;
//                 }
//                 Ok(None) => {}
//             }
//         }
//     }

//     #[test]
//     fn test_demux_decode_encode_mux_aac() {
//         let mut file = File::open("./xgplayer-demo-360p.flv").unwrap();

//         let mut buffer = Vec::new();
//         file.read_to_end(&mut buffer).expect("Failed to read file");

//         let bytes = BytesMut::from(&buffer[..]);

//         let mut flv_demuxer = FlvDemuxer::new(bytes);
//         flv_demuxer.read_flv_header().unwrap();

//         let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
//         let aac_encoder = AacEncoder::new(fdk_aac::enc::EncoderParams {
//             bit_rate: fdk_aac::enc::BitRate::VbrMedium,
//             sample_rate: 48000,
//             transport: fdk_aac::enc::Transport::Raw,
//             channels: fdk_aac::enc::ChannelMode::Stereo,
//         })
//         .unwrap();

//         let mut audio_demuxer = FlvAudioTagDemuxer::new();
//         let mut audio_pcm_data_from_decoded_aac: Vec<i16> = Vec::new();

//         let mut file = File::create("/Users/hailiang8/xgplayer-demo-360p-aac.flv").unwrap();

//         let mut flv_muxer = FlvMuxer::default();
//         flv_muxer.write_flv_header().unwrap();
//         flv_muxer.write_previous_tag_size(0).unwrap();

//         let mut time: u32 = 0;

//         loop {
//             match flv_demuxer.read_flv_tag() {
//                 Ok(Some(flv_data)) => {
//                     match flv_data {
//                         xflv::define::FlvData::Audio { timestamp, data } => {
//                             println!("audio: time:{}", timestamp);
//                             let len = data.len() as u32;

//                             match audio_demuxer.demux(timestamp, data.clone()) {
//                                 Ok(d) => {
//                                     if !d.has_data {
//                                         flv_muxer.write_flv_tag_header(8, len, timestamp).unwrap();
//                                         flv_muxer.write_flv_tag_body(data).unwrap();
//                                         flv_muxer.write_previous_tag_size(len + 11).unwrap();

//                                         let data = flv_muxer.writer.extract_current_bytes();
//                                         file.write_all(&data[..]).unwrap();

//                                         continue;
//                                     }

//                                     println!("aac demux len: {}", d.data.len());
//                                     aac_decoder.fill(&d.data).unwrap();
//                                     let mut decode_frame = vec![0_i16; 1024 * 3];

//                                     match aac_decoder.decode_frame(&mut decode_frame[..]) {
//                                         Ok(()) => {
//                                             let len = aac_decoder.decoded_frame_size();

//                                             audio_pcm_data_from_decoded_aac
//                                                 .extend_from_slice(&decode_frame[..len]);

//                                             while audio_pcm_data_from_decoded_aac.len() >= 2048 {
//                                                 let pcm =
//                                                     audio_pcm_data_from_decoded_aac.split_off(2048);

//                                                 let mut encoded_aac: Vec<u8> = vec![0; 1500];
//                                                 match aac_encoder.encode(
//                                                     &audio_pcm_data_from_decoded_aac,
//                                                     &mut encoded_aac[..],
//                                                 ) {
//                                                     Ok(info) => {
//                                                         println!("aac encode ok : {:?}", info);
//                                                         audio_pcm_data_from_decoded_aac = pcm;

//                                                         if info.output_size > 0 {
//                                                             let audio_tag_header = AudioTagHeader {
//                                                                 sound_format: 10,
//                                                                 sound_rate: 3,
//                                                                 sound_size: 1,
//                                                                 sound_type: 1,
//                                                                 aac_packet_type: 1,
//                                                             };

//                                                             let tag_header_data =
//                                                                 audio_tag_header.marshal().unwrap();

//                                                             let mut writer = BytesWriter::new();
//                                                             writer.write(&tag_header_data).unwrap();
//                                                             let audio_data =
//                                                                 &encoded_aac[..info.output_size];
//                                                             writer.write(audio_data).unwrap();

//                                                             let body =
//                                                                 writer.extract_current_bytes();

//                                                             let len = body.len() as u32;

//                                                             flv_muxer
//                                                                 .write_flv_tag_header(8, len, time)
//                                                                 .unwrap();
//                                                             flv_muxer
//                                                                 .write_flv_tag_body(body)
//                                                                 .unwrap();
//                                                             flv_muxer
//                                                                 .write_previous_tag_size(len + 11)
//                                                                 .unwrap();

//                                                             time += 21;

//                                                             let data = flv_muxer
//                                                                 .writer
//                                                                 .extract_current_bytes();
//                                                             file.write_all(&data[..]).unwrap();
//                                                         }
//                                                         if info.input_consumed > 0
//                                                             && info.output_size > 0
//                                                         {
//                                                         } else {
//                                                             break;
//                                                         }
//                                                     }
//                                                     Err(err) => {
//                                                         println!("aac encode err : {}", err);
//                                                     }
//                                                 }
//                                             }
//                                         }
//                                         Err(err) => {
//                                             println!("decoder error: {}", err);
//                                             // return;
//                                         }
//                                     }
//                                 }
//                                 Err(err) => {
//                                     println!("demux error: {}", err);
//                                 }
//                             }
//                         }
//                         xflv::define::FlvData::Video {
//                             timestamp: _,
//                             data: _,
//                         } => {
//                             println!("video");
//                         }
//                         xflv::define::FlvData::MetaData {
//                             timestamp: _,
//                             data: _,
//                         } => {
//                             println!("metadata");
//                         }
//                     }
//                 }
//                 Err(err) => {
//                     println!("read error: {}", err);
//                     // file.w
//                     return;
//                 }
//                 Ok(None) => {}
//             }
//         }
//     }
// }
