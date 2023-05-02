pub mod fmtp;

#[derive(Debug, Clone, Default)]
struct RtpMap {
    payload_type: u16,
    encoding_name: String,
    clock_rate: u32,
    encoding_parm: String,
}

impl RtpMap {
    pub fn parse(&mut self, raw_string: String) {
        let parts: Vec<&str> = raw_string.split(' ').collect();

        if let Ok(payload_type) = parts[0].parse::<u16>() {
            self.payload_type = payload_type;
        }

        let second_parts: Vec<&str> = parts[1].split('/').collect();
        let second_part_size = second_parts.len();

        if second_part_size > 0 {
            self.encoding_name = second_parts[0].to_string();
        }
        if second_part_size > 1 {
            if let Ok(clock_rate) = second_parts[1].parse::<u32>() {
                self.clock_rate = clock_rate;
            }
        }
        if second_part_size > 2 {
            self.encoding_parm = second_parts[2].to_string();
        }
    }
}

#[cfg(test)]
mod tests {

    use super::RtpMap;

    #[test]
    fn test_parse_rtpmap() {
        let mut parser = RtpMap::default();

        parser.parse("97 MPEG4-GENERIC/44100/2".to_string());

        println!(" parser: {:?}", parser);

        assert_eq!(parser.payload_type, 97);
        assert_eq!(parser.encoding_name, "MPEG4-GENERIC".to_string());
        assert_eq!(parser.clock_rate, 44100);
        assert_eq!(parser.encoding_parm, "2".to_string());

        let mut parser2 = RtpMap::default();
        parser2.parse("96 H264/90000".to_string());

        println!(" parser2: {:?}", parser2);

        assert_eq!(parser2.payload_type, 96);
        assert_eq!(parser2.encoding_name, "H264".to_string());
        assert_eq!(parser2.clock_rate, 90000);
        assert_eq!(parser2.encoding_parm, "".to_string());
    }
}
