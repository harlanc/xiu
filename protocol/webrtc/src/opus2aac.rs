use crate::errors::Opus2AacError;
use fdk_aac::enc::{Encoder as AacEncoder, EncoderParams};
use opus::Decoder as OpusDecoder;
pub struct Opus2AacTranscoder {
    decoder_sample_rate: u32,
    decoder_channels: opus::Channels,
    decoder: OpusDecoder,
    encoder_sample_rate: u32,
    encoder_channels: fdk_aac::enc::ChannelMode,
    encoder: AacEncoder,
}

impl Opus2AacTranscoder {
    pub fn new(
        decoder_sample_rate: u32,
        decoder_channels: opus::Channels,
        encoder_sample_rate: u32,
        encoder_channels: fdk_aac::enc::ChannelMode,
    ) -> Result<Self, Opus2AacError> {
        let decoder = OpusDecoder::new(decoder_sample_rate, decoder_channels)?;
        let encoder = AacEncoder::new(EncoderParams {
            bit_rate: fdk_aac::enc::BitRate::VbrMedium,
            transport: fdk_aac::enc::Transport::Raw,
            channels: encoder_channels,
            sample_rate: encoder_sample_rate,
        })?;

        Ok(Opus2AacTranscoder {
            decoder_sample_rate,
            decoder_channels,
            decoder,
            encoder_sample_rate,
            encoder_channels,
            encoder,
        })
    }

    pub fn transcode(&mut self, input: &[u8]) -> Result<Option<Vec<u8>>, Opus2AacError> {
        let mut pcm_output: Vec<i16> = vec![0; 1024 * 2];
        //https://opus-codec.org/docs/opus_api-1.1.2/group__opus__decoder.html#ga7d1111f64c36027ddcb81799df9b3fc9
        let pcm_output_len = self.decoder.decode(input, &mut pcm_output[..], false)?;

        let mut aac_output: Vec<u8> = vec![0; 1024 * 2];
        let channels = match self.decoder_channels {
            opus::Channels::Stereo => 2,
            opus::Channels::Mono => 1,
        };

        let encoder_info = self
            .encoder
            .encode(&pcm_output[..pcm_output_len * channels], &mut aac_output)?;

        if encoder_info.output_size > 0 {
            Ok(Some(aac_output[..encoder_info.output_size].to_vec()))
        } else {
            Ok(None)
        }
    }
}
#[cfg(test)]
mod tests {
    use opus::Decoder;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    #[test]
    fn send_dump_file() {
        // 打开 Opus 文件
        let mut opus_file = File::open("/Users/zexu/output.opus").expect("无法打开 Opus 文件");

        // 读取文件头部信息
        let mut header = [0; 19];
        opus_file
            .read_exact(&mut header)
            .expect("无法读取文件头部信息");

        // 初始化解码器
        let sample_rate = 48000;
        const channels: usize = 2;
        let mut decoder =
            Decoder::new(sample_rate, opus::Channels::Stereo).expect("无法初始化解码器");

        // 定位到数据部分的起始位置
        // opus_file
        //     .seek(SeekFrom::Start(19))
        //     .expect("无法定位到数据起始位置");

        // 解码并输出音频数据
        const frame_size: usize = 960;

        let mut pcm = [0i16; frame_size * channels];

        loop {
            println!("read size...");
            let mut data = vec![0u8; frame_size];
            let bytes_read = opus_file.read(&mut data).expect("读取数据错误");
            if bytes_read == 0 {
                // 读取完毕
                break;
            } else {
                println!("bytes_read : {}...", bytes_read);
            }

            let mut cur_idx = 0;

            loop {
                // 解码 Opus 数据
                match decoder.decode(&data[cur_idx..], &mut pcm, false) {
                    Ok(len) => {
                        println!("size: {}", len);
                        cur_idx += len;

                        if cur_idx >= 960 {
                            println!("break");
                            break;
                        }
                    }
                    Err(err) => {
                        print!("err :{}", err);
                        return;
                    }
                }
            }

            // 处理解码得到的音频数据（例如，将 pcm 数据写入文件或播放音频）
            // ...
        }
    }
}
