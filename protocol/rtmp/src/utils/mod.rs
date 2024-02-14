pub mod errors;
pub mod print;

use commonlib::scanf;
use errors::RtmpUrlParseError;
use errors::RtmpUrlParseErrorValue;
use indexmap::IndexMap;

#[derive(Debug, Clone, Default)]
pub struct RtmpUrlParser {
    pub url: String,
    // host_with_port = format!("{}:{}",host,port)
    pub host_with_port: String,
    pub host: String,
    pub port: Option<String>,
    pub app_name: String,
    // / = format!("{}?{}",stream_name,query)
    pub stream_name_with_query: String,
    pub stream_name: String,
    pub query: Option<String>,
}

impl RtmpUrlParser {
    pub fn new(url: String) -> Self {
        Self {
            url,
            ..Default::default()
        }
    }

    /*
     example;rtmp://domain.name.cn:1935/app_name/stream_name?auth_key=test_Key
     host_with_port: domain.name.cn:1935
     host: domain.name.cn
     port: 1935
     app_name: app_name
     stream_name_with_query: stream_name?auth_key=test_Key
     stream_name: stream_name
     query: auth_key=test_Key
    */
    pub fn parse_url(&mut self) -> Result<(), RtmpUrlParseError> {
        if let Some(idx) = self.url.find("rtmp://") {
            let remove_header_left = &self.url[idx + 7..];
            let url_parts: Vec<&str> = remove_header_left.split('/').collect();
            if url_parts.len() != 3 {
                return Err(RtmpUrlParseError {
                    value: RtmpUrlParseErrorValue::Notvalid,
                });
            }

            self.host_with_port = url_parts[0].to_string();
            self.app_name = url_parts[1].to_string();
            self.stream_name_with_query = url_parts[2].to_string();

            self.parse_host_with_port()?;
            (self.stream_name, self.query) =
                Self::parse_stream_name_with_query(&self.stream_name_with_query);
        } else {
            return Err(RtmpUrlParseError {
                value: RtmpUrlParseErrorValue::Notvalid,
            });
        }

        Ok(())
    }

    pub fn parse_host_with_port(&mut self) -> Result<(), RtmpUrlParseError> {
        let data: Vec<&str> = self.host_with_port.split(':').collect();
        self.host = data[0].to_string();
        if data.len() > 1 {
            self.port = Some(data[1].to_string());
        }
        Ok(())
    }
    /*parse the stream name and query to get real stream name and query*/
    pub fn parse_stream_name_with_query(stream_name_with_query: &str) -> (String, Option<String>) {
        let data: Vec<&str> = stream_name_with_query.split('?').collect();
        let stream_name = data[0].to_string();
        let query = if data.len() > 1 {
            let query_val = data[1].to_string();

            let mut query_pairs = IndexMap::new();
            let pars_array: Vec<&str> = query_val.split('&').collect();
            for ele in pars_array {
                let (k, v) = scanf!(ele, '=', String, String);
                if k.is_none() || v.is_none() {
                    continue;
                }
                query_pairs.insert(k.unwrap(), v.unwrap());
            }
            Some(data[1].to_string())
        } else {
            None
        };
        (stream_name, query)
    }

    pub fn append_port(&mut self, port: String) {
        if !self.host_with_port.contains(':') {
            self.host_with_port = format!("{}:{}", self.host_with_port, port);
            self.port = Some(port);
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

        println!(" raw_domain_name: {}", parser.host_with_port);
        if parser.port.is_some() {
            println!(" port: {}", parser.port.unwrap());
        }
        println!(" domain_name: {}", parser.host);
        println!(" app_name: {}", parser.app_name);
        println!(" stream_name_with_query: {}", parser.stream_name_with_query);
        println!(" stream_name: {}", parser.stream_name);
        if parser.query.is_some() {
            println!(" query: {}", parser.query.unwrap());
        }
    }
    #[test]
    fn test_rtmp_url_parser2() {
        let mut parser =
            RtmpUrlParser::new(String::from("rtmp://domain.name.cn/app_name/stream_name"));

        parser.parse_url().unwrap();

        println!(" raw_domain_name: {}", parser.host_with_port);
        if parser.port.is_some() {
            println!(" port: {}", parser.port.unwrap());
        }
        println!(" domain_name: {}", parser.host);
        println!(" app_name: {}", parser.app_name);
        println!(" stream_name_with_query: {}", parser.stream_name_with_query);
        println!(" stream_name: {}", parser.stream_name);
        if parser.query.is_some() {
            println!(" query: {}", parser.query.unwrap());
        }
    }
}
