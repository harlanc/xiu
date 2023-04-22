pub mod errors;
pub mod print;

use errors::RtmpUrlParseError;
use errors::RtmpUrlParseErrorValue;

#[derive(Debug, Clone, Default)]
pub struct RtmpUrlParser {
    pub raw_url: String,
    // raw_domain_name = format!("{}:{}",domain_name,port)
    pub raw_domain_name: String,
    pub domain_name: String,
    pub port: String,
    pub app_name: String,
    // raw_stream_name = format!("{}?{}",stream_name,url_parameters)
    pub raw_stream_name: String,
    pub stream_name: String,
    pub parameters: String,
}

impl RtmpUrlParser {
    pub fn new(url: String) -> Self {
        Self {
            raw_url: url,
            ..Default::default()
        }
    }

    pub fn set_raw_stream_name(&mut self, raw_stream_name: String) -> &mut Self {
        self.raw_stream_name = raw_stream_name;
        self
    }

    /*
     example;rtmp://domain.name.cn:1935/app_name/stream_name?auth_key=test_Key
     raw_domain_name: domain.name.cn:1935
     domain_name: domain.name.cn
     app_name: app_name
     raw_stream_name: stream_name?auth_key=test_Key
     stream_name: stream_name
     url_parameters: auth_key=test_Key
    */
    pub fn parse_url(&mut self) -> Result<(), RtmpUrlParseError> {
        if let Some(idx) = self.raw_url.find("rtmp://") {
            let remove_header_left = &self.raw_url[idx + 7..];
            let url_parts: Vec<&str> = remove_header_left.split('/').collect();
            if url_parts.len() != 3 {
                return Err(RtmpUrlParseError {
                    value: RtmpUrlParseErrorValue::Notvalid,
                });
            }

            self.raw_domain_name = url_parts[0].to_string();
            self.app_name = url_parts[1].to_string();
            self.raw_stream_name = url_parts[2].to_string();

            self.parse_raw_domain_name()?;
            self.parse_raw_stream_name();
        } else {
            return Err(RtmpUrlParseError {
                value: RtmpUrlParseErrorValue::Notvalid,
            });
        }

        Ok(())
    }

    pub fn parse_raw_domain_name(&mut self) -> Result<(), RtmpUrlParseError> {
        let data: Vec<&str> = self.raw_domain_name.split(':').collect();
        self.domain_name = data[0].to_string();
        if data.len() > 1 {
            self.port = data[1].to_string();
        }
        Ok(())
    }
    /*parse the raw stream name to get real stream name and the URL parameters*/
    pub fn parse_raw_stream_name(&mut self) -> (String, String) {
        let data: Vec<&str> = self.raw_stream_name.split('?').collect();
        self.stream_name = data[0].to_string();
        if data.len() > 1 {
            self.parameters = data[1].to_string();
        }
        (self.stream_name.clone(), self.parameters.clone())
    }

    pub fn append_port(&mut self, port: String) {
        if !self.raw_domain_name.contains(':') {
            self.raw_domain_name = format!("{}:{}", self.raw_domain_name, port);
            self.port = port;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::RtmpUrlParser;
    #[test]
    fn test_rtmp_url_parser() {
        let mut parser = RtmpUrlParser::new(String::from(
            "rtmp://domain.name.cn:1935/app_name/stream_name?auth_key=test_Key",
        ));

        parser.parse_url().unwrap();

        println!(" raw_domain_name: {}", parser.raw_domain_name);
        println!(" port: {}", parser.port);
        println!(" domain_name: {}", parser.domain_name);
        println!(" app_name: {}", parser.app_name);
        println!(" raw_stream_name: {}", parser.raw_stream_name);
        println!(" stream_name: {}", parser.stream_name);
        println!(" url_parameters: {}", parser.parameters);
    }
    #[test]
    fn test_rtmp_url_parser2() {
        let mut parser =
            RtmpUrlParser::new(String::from("rtmp://domain.name.cn/app_name/stream_name"));

        parser.parse_url().unwrap();

        println!(" raw_domain_name: {}", parser.raw_domain_name);
        println!(" port: {}", parser.port);
        println!(" domain_name: {}", parser.domain_name);
        println!(" app_name: {}", parser.app_name);
        println!(" raw_stream_name: {}", parser.raw_stream_name);
        println!(" stream_name: {}", parser.stream_name);
        println!(" url_parameters: {}", parser.parameters);
    }
}
