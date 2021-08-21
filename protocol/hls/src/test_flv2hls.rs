#[cfg(test)]
mod tests {
    use crate::errors::MediaError;
    use crate::flv2hls::Flv2HlsRemuxer;
    use bytes::BytesMut;
    use xflv::define::FlvData;

    use xflv::demuxer::FlvDemuxer;

    use std::fs::File;
    use std::io::prelude::*;
    use std::time::Instant;

    #[allow(dead_code)]
    pub fn print(data: BytesMut) {
        print!("==========={}\n", data.len());
        let mut idx = 0;
        for i in data {
            print!("{:02X} ", i);
            idx = idx + 1;
            match idx % 16 {
                0 => {
                    print!("\n")
                }
                _ => {}
            }
        }

        print!("===========\n")
    }
    #[allow(dead_code)]
    pub fn print_flv_data(data: FlvData) {
        match data {
            FlvData::Audio { timestamp, data } => {
                print! {"==audio data begin==\n"};
                print! {"timestamp: {}\n",timestamp};
                print! {"data :\n"};
                print(data);
                print! {"==audio data end==\n"};
            }
            FlvData::Video { timestamp, data } => {
                print! {"==video data begin==\n"};
                print! {"timestamp: {}\n",timestamp};
                print! {"data :\n"};
                print(data);
                print! {"==video data end==\n"};
            }
            _ => {
                print!("not video or audio \n")
            }
        }
    }

    //#[test]
    #[allow(dead_code)]
    fn test_flv2hls() -> Result<(), MediaError> {
        let mut file =
            File::open("/Users/zexu/github/xiu/protocol/hls/src/xgplayer_demo.flv").unwrap();
        let mut contents = Vec::new();

        file.read_to_end(&mut contents)?;

        let mut data = BytesMut::new();
        data.extend(contents);

        let mut demuxer = FlvDemuxer::new(data);
        demuxer.read_flv_header()?;

        let start = Instant::now();
        let mut media_demuxer = Flv2HlsRemuxer::new(5, String::from("live"), String::from("test"));

        loop {
            let data_ = demuxer.read_flv_tag();

            match data_ {
                Ok(data) => {
                    if let Some(real_data) = data {
                        media_demuxer.process_flv_data(real_data)?;
                    }
                }

                Err(err) => {
                    media_demuxer.flush_remaining_data()?;
                    log::error!("readd err: {}", err);
                    break;
                }
            }
        }

        let elapsed = start.elapsed();
        println!("Debug: {:?}", elapsed);

        Ok(())
    }
}
