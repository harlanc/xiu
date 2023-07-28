use crate::global_trait::{Marshal, Unmarshal};

#[derive(Debug, Clone, Default)]
pub struct RtpMap {
    pub payload_type: u16,
    pub encoding_name: String,
    pub clock_rate: u32,
    pub encoding_param: String,
}

impl Unmarshal for RtpMap {
    // a=rtpmap:96 H264/90000\r\n\
    // a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\

    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtpmap = RtpMap::default();

        let parts: Vec<&str> = raw_data.split(' ').collect();

        if let Some(part_0) = parts.first() {
            if let Ok(payload_type) = part_0.parse::<u16>() {
                rtpmap.payload_type = payload_type;
            }
        }

        if let Some(part_1) = parts.get(1) {
            let parameters: Vec<&str> = part_1.split('/').collect();

            if let Some(para_0) = parameters.first() {
                rtpmap.encoding_name = para_0.to_string();
            }

            if let Some(para_1) = parameters.get(1) {
                if let Ok(clock_rate) = para_1.parse::<u32>() {
                    rtpmap.clock_rate = clock_rate;
                }
            }
            if let Some(para_2) = parameters.get(2) {
                rtpmap.encoding_param = para_2.to_string();
            }
        }

        Some(rtpmap)
    }
}

impl Marshal for RtpMap {
    fn marshal(&self) -> String {
        let mut rtpmap = format!(
            "{} {}/{}",
            self.payload_type, self.encoding_name, self.clock_rate
        );
        if self.encoding_param != *"" {
            rtpmap = format!("{}/{}", rtpmap, self.encoding_param);
        }

        format!("{rtpmap}\r\n")
    }
}

#[cfg(test)]
mod tests {

    use crate::global_trait::{Marshal, Unmarshal};

    use super::RtpMap;

    #[test]
    fn test_marshal_unmarshal_rtpmap() {
        let parser = RtpMap::unmarshal("97 MPEG4-GENERIC/44100/2").unwrap();

        println!(" parser: {parser:?}");

        assert_eq!(parser.payload_type, 97);
        assert_eq!(parser.encoding_name, "MPEG4-GENERIC".to_string());
        assert_eq!(parser.clock_rate, 44100);
        assert_eq!(parser.encoding_param, "2".to_string());

        print!("marshal str:{}", parser.marshal());

        let parser2 = RtpMap::unmarshal("96 H264/90000").unwrap();

        println!(" parser2: {parser2:?}");

        assert_eq!(parser2.payload_type, 96);
        assert_eq!(parser2.encoding_name, "H264".to_string());
        assert_eq!(parser2.clock_rate, 90000);
        assert_eq!(parser2.encoding_param, "".to_string());

        print!("marshal str2 :{}", parser2.marshal());
    }
}
