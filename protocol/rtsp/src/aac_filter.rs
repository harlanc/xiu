use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use fdk_aac::dec::Decoder as AacDecoder;
use fdk_aac::enc::{AudioObjectType, Encoder as AacEncoder, EncoderParams};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use webrtc_audio_processing as apm;

const APM_FRAME_SIZE: usize = 480;
const ENCODE_FRAME_SIZE: usize = 1024;
pub struct AacFilter {
    decoder: AacDecoder,
    resampler_in: FastFixedIn<f32>,
    resampler_out: FastFixedIn<f32>,
    processor: apm::Processor,
    encoder: AacEncoder,
    stack_in: Vec<f32>,
    stack_out: Vec<f32>,
    cache: Vec<i16>,
    // process_offset: usize,
    frame_size: usize,
    sample_rate: i32,
    channels: u8,
}
impl AacFilter {
    pub fn new(sample_rate: i32, channels: u8) -> Result<Self> {
        let frame_size = 1024; // LC

        let config = apm::InitializationConfig {
            num_capture_channels: channels as i32, // Stereo mic input
            num_render_channels: channels as i32,  // Stereo speaker output
            ..apm::InitializationConfig::default()
        };
        let mut ap = apm::Processor::new(&config).unwrap();
        let config = apm::Config {
            echo_cancellation: None,
            gain_control: None,
            noise_suppression: Some(apm::NoiseSuppression {
                suppression_level: apm::NoiseSuppressionLevel::High,
            }),
            enable_high_pass_filter: true,
            ..apm::Config::default()
        };
        ap.set_config(config);

        let mut decoder = AacDecoder::new(fdk_aac::dec::Transport::Raw);

        // 默认都是LC
        let asc_audio_object_type = 2u16;
        let asc_sampling_frequency_index = match sample_rate {
            48000 => 0x3,
            44100 => 0x4,
            16000 => 0x8,
            8000 => 0xb,
            _ => return Err(anyhow!("sample_rate {sample_rate} not yet supported")),
        } as u16;
        let asc_channels = match channels {
            1 => 0x1,
            2 => 0x2,
            _ => return Err(anyhow!("channels {channels} not yet supported")),
        } as u16;
        let asc =
            asc_audio_object_type << 11 | asc_sampling_frequency_index << 7 | asc_channels << 3;
        decoder
            .config_raw(&asc.to_be_bytes())
            .map_err(|e| anyhow!("{e:?}"))?;
        decoder
            .set_max_output_channels(channels as usize)
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Self {
            decoder,
            resampler_in: FastFixedIn::<f32>::new(
                48000 as f64 / sample_rate as f64,
                100.0,
                PolynomialDegree::Cubic,
                frame_size,
                channels as usize,
            )?,
            resampler_out: FastFixedIn::<f32>::new(
                sample_rate as f64 / 48000 as f64,
                100.0,
                PolynomialDegree::Cubic,
                frame_size,
                channels as usize,
            )?,
            processor: ap,
            encoder: AacEncoder::new(EncoderParams {
                bit_rate: fdk_aac::enc::BitRate::VbrLow,
                transport: fdk_aac::enc::Transport::Adts,
                channels: if channels == 2 {
                    fdk_aac::enc::ChannelMode::Stereo
                } else if channels == 1 {
                    fdk_aac::enc::ChannelMode::Mono
                } else {
                    return Err(anyhow!("channels {channels} not support for apm"));
                },
                audio_object_type: AudioObjectType::Mpeg4LowComplexity,
                sample_rate: sample_rate as u32,
            })
            .map_err(|e| anyhow!("{e:?}"))?,
            stack_in: vec![],
            stack_out: vec![],
            cache: vec![],
            // process_offset: 0,
            frame_size,
            sample_rate,
            channels,
        })
    }

    pub fn set_decoder_asc(&mut self, asc: u16) -> Result<()> {
        Ok(self
            .decoder
            .config_raw(&asc.to_be_bytes())
            .map_err(|e| anyhow!("{e:?}"))?)
    }

    pub fn pipeline(&mut self, data: &[u8]) -> Result<BytesMut> {
        // 将编码数据解码并压到处理栈中，压入时进行上采样
        let mut i = 0;
        let mut sample_buf = vec![0; self.frame_size];
        while i < data.len() {
            let consumed = self.decoder.fill(data).map_err(|e| anyhow!("{e:?}"))?;
            self.decoder
                .decode_frame(&mut sample_buf)
                .map_err(|e| anyhow!("{e:?}"))?;
            // webrtc apm只支持48000hz
            if self.sample_rate != 48000 {
                let fltp_samples = s16_to_fltp(&sample_buf);
                let resampled = self.resampler_in.process(&vec![fltp_samples], None)?;
                self.stack_in.extend_from_slice(&intervene(&resampled)?);
            } else {
                self.stack_in.extend_from_slice(&s16_to_fltp(&sample_buf));
            }

            i += consumed;
        }

        // 将处理栈中的数据按process_size进行音色过滤，注意这里只能是48000采样率的/10ms
        // 过滤后根据采样率可选进行下采样恢复成原来的采样率，然后压入编码栈
        while self.stack_in.len() >= APM_FRAME_SIZE {
            let new_stack_in = self.stack_in.split_off(APM_FRAME_SIZE);
            if let Err(e) = self.processor.process_capture_frame(&mut self.stack_in) {
                log::warn!("process err: {e:?}");
            }
            self.stack_out.append(&mut self.stack_in);

            self.stack_in = new_stack_in;
        }

        // 根据编码栈的采样进行对LC下的样本进行编码，合并成输出aac数据
        // 栈数据处理结构图
        // |              offset1              |
        // +-----------------------------------------+
        // | 480 | 480 | 480 | 480 | 480 | 480 | pad |
        // +-----------------------------------------+
        // |    1024    |    1024    | tobe encode   |
        // +-----------------------------------------+
        // |                         | offset2 |
        let mut result = BytesMut::new();
        while self.stack_out.len() >= ENCODE_FRAME_SIZE {
            let new_stack_out = self.stack_out.split_off(ENCODE_FRAME_SIZE);
            if self.sample_rate != 48000 {
                let resampled = self.resampler_out.process(&vec![&self.stack_out], None)?;
                let mut resampled_intervene = fltp_to_s16(&intervene(&resampled)?);
                self.cache.append(&mut resampled_intervene);
            } else {
                self.cache.append(&mut fltp_to_s16(&self.stack_out));
            }
            self.stack_out = new_stack_out;
        }

        let mut aac_output: Vec<u8> = vec![0; ENCODE_FRAME_SIZE * 2];
        while self.cache.len() >= ENCODE_FRAME_SIZE {
            let new_cache = self.cache.split_off(ENCODE_FRAME_SIZE);
            if let Ok(info) = self.encoder.encode(&self.cache, &mut aac_output) {
                let out_size = info.output_size;
                if out_size > 0 {
                    result.extend_from_slice(&aac_output[..out_size]);
                }
            }
            self.cache = new_cache;
        }

        Ok(result)
    }

    pub fn flush(&mut self) -> Result<Bytes> {
        // flush 用于最后清空栈中的数据，stack_in这时不到一个480，所以直接处理到编码栈中
        if self.stack_in.len() > 0 {
            self.stack_out.append(&mut self.stack_in);
        }

        if self.stack_out.len() > 0 {
            if self.sample_rate != 48000 {
                let resampled = self.resampler_out.process(&vec![&self.stack_out], None)?;
                let mut resampled_intervene = fltp_to_s16(&intervene(&resampled)?);
                self.cache.append(&mut resampled_intervene);
            } else {
                self.cache.append(&mut fltp_to_s16(&self.stack_out));
            }
        }

        let mut result = BytesMut::new();
        // 检查编码栈，尝试凑足1024个采样进行编码
        let mut aac_output: Vec<u8> = vec![0; ENCODE_FRAME_SIZE * 2];
        while self.cache.len() >= ENCODE_FRAME_SIZE {
            let new_cache = self.cache.split_off(ENCODE_FRAME_SIZE);
            if let Ok(info) = self.encoder.encode(&self.cache, &mut aac_output) {
                let out_size = info.output_size;
                if out_size > 0 {
                    result.extend_from_slice(&aac_output[..out_size]);
                }
            }
            self.cache = new_cache;
        }
        // 检查编码栈，将最后不够1024个采样进行编码
        if self.cache.len() > 0 {
            if let Ok(info) = self.encoder.encode(&self.cache, &mut aac_output) {
                let out_size = info.output_size;
                if out_size > 0 {
                    result.extend_from_slice(&aac_output[..out_size]);
                }
            }
        }
        Ok(result.freeze())
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;
    use std::io::Write;

    use crate::aac_filter::AacDecoder;
    use crate::aac_filter::AacFilter;
    use adts_reader::AdtsConsumer;
    use adts_reader::AdtsParseError;
    use adts_reader::AdtsParser;
    use adts_reader::AudioObjectType;
    use adts_reader::ChannelConfiguration;
    use adts_reader::MpegVersion;
    use adts_reader::Originality;
    use adts_reader::ProtectionIndicator;
    use adts_reader::SamplingFrequency;
    use anyhow::Result;
    struct DumpAdtsConsumer {
        frame_count: usize,
        filter: AacFilter,
        writer: BufWriter<std::fs::File>,
        asc: u16,
    }
    impl DumpAdtsConsumer {
        fn new(sample_rate: i32, channels: u8, output: &str) -> Result<Self> {
            Ok(Self {
                frame_count: 0,
                filter: AacFilter::new(sample_rate, channels)?,
                writer: BufWriter::new(std::fs::File::create(output)?),
                asc: 0,
            })
        }
        fn flush(&mut self) -> Result<()> {
            let _ = self.writer.write(&self.filter.flush()?);
            Ok(())
        }
    }
    impl AdtsConsumer for DumpAdtsConsumer {
        fn new_config(
            &mut self,
            mpeg_version: MpegVersion,
            protection: ProtectionIndicator,
            aot: AudioObjectType,
            freq: SamplingFrequency,
            private_bit: u8,
            ch: ChannelConfiguration,
            originality: Originality,
            home: u8,
        ) {
            println!("New ADTS configuration found");
            println!(
                "{:?} {:?} {:?} {:?} private_bit={} {:?} {:?} home={}",
                mpeg_version, protection, aot, freq, private_bit, ch, originality, home
            );
            // audio_object_type 5bits
            let audio_object_type = match aot {
                AudioObjectType::AacMain => 1u8,
                AudioObjectType::AacLC => 2,
                AudioObjectType::AacSSR => 3,
                AudioObjectType::AacLTP => 4,
            } as u16;
            // samplingFrequencyIndex 4 bits
            let sampling_frequency_index = match freq {
                SamplingFrequency::Freq96000 => Some(0x0u8),
                SamplingFrequency::Freq88200 => Some(0x1),
                SamplingFrequency::Freq64000 => Some(0x2),
                SamplingFrequency::Freq48000 => Some(0x3),
                SamplingFrequency::Freq44100 => Some(0x4),
                SamplingFrequency::Freq32000 => Some(0x5),
                SamplingFrequency::Freq24000 => Some(0x6),
                SamplingFrequency::Freq22050 => Some(0x7),
                SamplingFrequency::Freq16000 => Some(0x8),
                SamplingFrequency::Freq12000 => Some(0x9),
                SamplingFrequency::Freq11025 => Some(0xa),
                SamplingFrequency::Freq8000 => Some(0xb),
                _ => None,
            }
            .map(|v| v as u16);
            let channels = match ch {
                ChannelConfiguration::ObjectTypeSpecificConfig => 0x0u8,
                ChannelConfiguration::Mono => 0x1,
                ChannelConfiguration::Stereo => 0x2,
                ChannelConfiguration::Three => 0x3,
                ChannelConfiguration::Four => 0x4,
                ChannelConfiguration::Five => 0x5,
                ChannelConfiguration::FiveOne => 0x6,
                ChannelConfiguration::SevenOne => 0x7,
            } as u16;
            if let Some(sfi) = sampling_frequency_index {
                self.asc = audio_object_type << 11 | sfi << 7 | channels << 3;
                let _ = self.filter.set_decoder_asc(self.asc);
            }
        }
        fn payload(&mut self, buffer_fullness: u16, number_of_blocks: u8, buf: &[u8]) {
            println!(
                "ADTS Frame buffer_fullness={} blocks={} buf={}",
                buffer_fullness,
                number_of_blocks,
                buf.len()
            );
            if let Ok(data) = self.filter.pipeline(buf).map_err(|e| {
                println!("{e:?}");
                e
            }) {
                let _ = self.writer.write(&data);
            }
            self.frame_count += 1;
        }
        fn error(&mut self, err: AdtsParseError) {
            println!("Error: {:?}", err);
        }
    }

    #[test]
    fn test_transcode() {
        use std::cmp;
        use std::io::{BufReader, BufWriter, Read, Write};
        // let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
        let mut f = BufReader::new(
            std::fs::File::open("/data/workspace/boe/rtxp_server/temp.aac").unwrap(),
        );
        // let mut data = vec![];
        // let _ = f.read_to_end(&mut data);

        const LEN: usize = 1024 * 1024;
        let mut buf = [0u8; LEN];
        let mut parser = AdtsParser::new(
            DumpAdtsConsumer::new(16000, 1, "/data/workspace/boe/rtxp_server/transcode.aac")
                .unwrap(),
        );
        loop {
            match f.read(&mut buf[..]) {
                Err(e) => panic!("{e:?}"),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    let target = &mut buf[0..n];
                    parser.push(target);
                }
            };
        }
        // let _ = parser.consumer.flush();
    }

    #[test]
    fn test_decode() {
        use std::cmp;
        use std::io::{BufReader, BufWriter, Read, Write};
        let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
        let mut f = BufReader::new(
            std::fs::File::open("/data/workspace/boe/rtxp_server/output.aac").unwrap(),
        );
        let mut data = vec![];
        let _ = f.read_to_end(&mut data);
        let mut decode_frame = vec![0; 1024];

        let mut writer = BufWriter::new(
            std::fs::File::create("/data/workspace/boe/rtxp_server/test.pcm").unwrap(),
        );

        let mut i = 0;
        while data.len() > 0 {
            match aac_decoder.fill(&data) {
                Ok(n) => {
                    println!("filled {n} {i}",);
                    let dec_result = aac_decoder.decode_frame(&mut decode_frame[..]);
                    println!("dec_result {dec_result:?} ",);
                    let write_result = writer.write(bytemuck::cast_slice::<i16, u8>(&decode_frame));
                    println!("write_result {write_result:?}",);
                    let remains = data.split_off(n);
                    data = remains;
                    println!("remains {}", data.len());
                }
                Err(e) => {
                    println!("fill_err {:?}", e.to_string());
                    return;
                }
            }
            i += 1;
        }
    }
}

// #[test]
// fn test_myfilter() {
//     const SAMPLE_RATE_IN: u32 = 16000;
//     const SAMPLE_RATE_OUT: u32 = 48000;
//     const CHANNELS: usize = 1;
//     const FRAME_SIZE: usize = 1024;
//     const FILTER_SAMPLES: usize = SAMPLE_RATE_OUT as usize / 100;
//     // use crate::utils;
//     use std::io::Write;
//     let _ = std::fs::remove_file("./output.wav");

//     let encoder = AacEncoder::new(EncoderParams {
//         bit_rate: fdk_aac::enc::BitRate::VbrLow,
//         transport: fdk_aac::enc::Transport::Adts,
//         channels: fdk_aac::enc::ChannelMode::Mono,
//         sample_rate: SAMPLE_RATE_OUT,
//     })
//     .unwrap();

//     let mut resampler = FastFixedIn::<f32>::new(
//         SAMPLE_RATE_OUT as f64 / SAMPLE_RATE_IN as f64,
//         100.0,
//         PolynomialDegree::Cubic,
//         FRAME_SIZE,
//         CHANNELS,
//     )
//     .unwrap();

//     let config = apm::InitializationConfig {
//         num_capture_channels: CHANNELS as i32, // Stereo mic input
//         num_render_channels: CHANNELS as i32,  // Stereo speaker output
//         ..apm::InitializationConfig::default()
//     };
//     let mut ap = apm::Processor::new(&config).unwrap();
//     let config = apm::Config {
//         echo_cancellation: None,
//         gain_control: None,
//         noise_suppression: Some(apm::NoiseSuppression {
//             suppression_level: apm::NoiseSuppressionLevel::High,
//         }),
//         enable_high_pass_filter: true,
//         ..apm::Config::default()
//     };
//     ap.set_config(config);

//     let mut reader =
//         hound::WavReader::open("/data/workspace/boe/DigtalTalk/media/noise.wav").unwrap();
//     let mut writer = BufWriter::new(std::fs::File::create("./output.wav").unwrap());
//     let samples = reader
//         .samples::<i16>()
//         .filter(|v| v.is_ok())
//         .map(|v| v.unwrap() as f32 / 0x8000 as f32)
//         .collect::<Vec<f32>>();
//     let chunk_samples = utils::chunks(&samples, FRAME_SIZE);
//     let mut resampled = vec![];
//     for cs in chunk_samples {
//         if let Ok(data) = resampler.process(&vec![cs], None) {
//             resampled.extend_from_slice(&data[0]);
//         }
//     }

//     for cr in resampled.chunks_exact_mut(FILTER_SAMPLES) {
//         println!("processing {}", cr.len());
//         if let Err(e) = ap.process_capture_frame(cr) {
//             println!("apm process_capture_frame Err {}", e.to_string());
//         };
//     }

//     // let _ = writer.write(bytemuck::cast_slice::<f32, u8>(&resampled));

//     let resampled_short = resampled
//         .into_iter()
//         .map(|v| (v * 0x8000 as f32) as i16)
//         .collect::<Vec<i16>>();

//     let mut aac_output: Vec<u8> = vec![0; FRAME_SIZE * 2];
//     let chunk_resampled = utils::chunks(&resampled_short, FRAME_SIZE);

//     for cr in chunk_resampled {
//         // println!("cr len {} {:?}", cr.len(), cr[..4].to_vec());
//         match encoder.encode(cr, &mut aac_output) {
//             Ok(encoder_info) => {
//                 // println!("cr encoder_info.output_size {}", encoder_info.output_size);
//                 if encoder_info.output_size > 0 {
//                     let _ = writer.write(&aac_output[..encoder_info.output_size]);
//                     // aac_output.clear();
//                 }
//             }
//             Err(e) => {
//                 println!("encoder Err {}", e.to_string());
//             }
//         }
//         // if let encoder_info = encoder.encode(cr, &mut aac_output);
//         // println!("encoded {}", encoder_info.output_size);
//     }
// }

// // use apm::{
// //     AudioProcessing, Config, EchoCancellation, EchoCancellationSuppressionLevel,
// //     InitializationConfig, NoiseSuppression, Processor,
// // };

// // pub struct AudioProcessor {
// //     processor: AudioProcessing,
// //     // stream_config: StreamConfig,
// // }

// // impl AudioProcessor {
// //     pub fn new(sample_rate: u32, channels: usize) -> Result<Self, Box<dyn std::error::Error>> {
// //         let mut processor = Processor::new(&InitializationConfig {
// //             num_capture_channels: channels as i32,
// //             num_render_channels: channels as i32,
// //             ..Default::default()
// //         })?;
// //         let config = Config {
// //             echo_cancellation: Some(EchoCancellation {
// //                 suppression_level: EchoCancellationSuppressionLevel::High,
// //                 enable_delay_agnostic: false,
// //                 enable_extended_filter: false,
// //                 stream_delay_ms: None,
// //             }),
// //             noise_suppression: Some(NoiseSuppression {
// //                 suppression_level: apm::NoiseSuppressionLevel::Moderate,
// //             }),
// //             ..Default::default()
// //         };
// //         processor.set_config(config);

// //         // processor.process_capture_frame(frame)
// //         // processor.process_render_frame(frame)

// //         let processor = AudioProcessing::new(config)?;

// //         // let stream_config = StreamConfig::new(sample_rate, channels);

// //         Ok(AudioProcessor {
// //             processor,
// //             // stream_config,
// //         })
// //     }

// //     /// 处理远端音频（建立参考信号）
// //     pub fn process_reverse_stream(
// //         &mut self,
// //         audio_data: &[i16],
// //     ) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
// //         self.processor
// //             .process_reverse_stream(audio_data, self.stream_config)?;
// //         Ok(audio_data.to_vec())
// //     }

// //     /// 处理近端音频（应用回声消除和噪音抑制）
// //     pub fn process_stream(
// //         &mut self,
// //         audio_data: &[i16],
// //     ) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
// //         let mut processed = vec![0i16; audio_data.len()];
// //         self.processor
// //             .process_stream(audio_data, &mut processed, self.stream_config)?;
// //         Ok(processed)
// //     }
// // }

pub fn chunks<T>(data: &[T], chunk_size: usize) -> Vec<&[T]> {
    let list = data.chunks_exact(chunk_size);
    let remains = list.remainder();
    let mut chunks = list.collect::<Vec<&[T]>>();
    if remains.len() > 0 {
        chunks.push(remains);
    }
    return chunks;
}

pub fn fltp_to_s16(data: &Vec<f32>) -> Vec<i16> {
    data.iter().map(|v| (*v * 0x8000 as f32) as i16).collect()
}
pub fn s16_to_fltp(data: &Vec<i16>) -> Vec<f32> {
    data.iter().map(|v| *v as f32 / 0x8000 as f32).collect()
}

pub fn intervene<T>(tracks: &Vec<Vec<T>>) -> Result<Vec<T>>
where
    T: Clone + Copy,
{
    let track_len = tracks.len();
    if track_len == 0 {
        return Err(anyhow!("no tracks"));
    }
    let data_len = tracks[0].len();
    if track_len == 1 {
        return Ok(tracks[0].clone());
    }
    if tracks.iter().find(|v| v.len() != data_len).is_some() {
        return Err(anyhow!("tracks len mismatch"));
    }
    Ok(tracks[0]
        .iter()
        .zip(tracks[1].iter())
        .flat_map(|(l, r)| std::iter::once(*l).chain(std::iter::once(*r)))
        .collect::<Vec<T>>())
}
