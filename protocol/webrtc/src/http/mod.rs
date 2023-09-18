pub mod define;
use indexmap::IndexMap;

macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

pub trait Unmarshal {
    fn unmarshal(request_data: &str) -> Option<Self>
    where
        Self: Sized;
}

pub trait Marshal {
    fn marshal(&self) -> String;
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub address: String,
    pub port: u16,
    pub path: String,
    pub path_parameters: Option<String>,
    //parse path_parameters and save the results
    pub path_parameters_map: IndexMap<String, String>,
    pub version: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn get_header(&self, header_name: &String) -> Option<&String> {
        self.headers.get(header_name)
    }
}

impl Unmarshal for HttpRequest {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut http_request = HttpRequest::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            //POST /whip?app=live&stream=test HTTP/1.1
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();
                if let Some(method) = fields.next() {
                    http_request.method = method.to_string();
                }
                if let Some(path) = fields.next() {
                    let path_data: Vec<&str> = path.splitn(2, '?').collect();
                    http_request.path = path_data[0].to_string();

                    if path_data.len() > 1 {
                        let pars = path_data[1].to_string();
                        let pars_array: Vec<&str> = pars.split('&').collect();

                        for ele in pars_array {
                            let (k, v) = scanf!(ele, '=', String, String);
                            if k.is_none() || v.is_none() {
                                continue;
                            }
                            http_request
                                .path_parameters_map
                                .insert(k.unwrap(), v.unwrap());
                        }
                        http_request.path_parameters = Some(pars);
                    }
                }
                if let Some(version) = fields.next() {
                    http_request.version = version.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    if name == "Host" {
                        let (address_val, port_val) = scanf!(value, ':', String, u16);
                        if let Some(address) = address_val {
                            http_request.address = address;
                        }
                        if let Some(port) = port_val {
                            http_request.port = port;
                        }
                    }
                    http_request.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            http_request.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(http_request)
    }
}

impl Marshal for HttpRequest {
    fn marshal(&self) -> String {
        let full_path = if let Some(parameters) = &self.path_parameters {
            format!("{}?{}", self.path, parameters)
        } else {
            self.path.clone()
        };
        let mut request_str = format!("{} {} {}\r\n", self.method, full_path, self.version);
        for (header_name, header_value) in &self.headers {
            if header_name != &"Content-Length".to_string() {
                request_str += &format!("{header_name}: {header_value}\r\n");
            }
        }
        if let Some(body) = &self.body {
            request_str += &format!("Content-Length: {}\r\n", body.len());
        }
        request_str += "\r\n";
        if let Some(body) = &self.body {
            request_str += body;
        }
        request_str
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl Unmarshal for HttpResponse {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut http_response = HttpResponse::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();

                if let Some(version) = fields.next() {
                    http_response.version = version.to_string();
                }
                if let Some(status) = fields.next() {
                    if let Ok(status) = status.parse::<u16>() {
                        http_response.status_code = status;
                    }
                }
                if let Some(reason_phrase) = fields.next() {
                    http_response.reason_phrase = reason_phrase.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    http_response.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            http_response.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(http_response)
    }
}

impl Marshal for HttpResponse {
    fn marshal(&self) -> String {
        let mut response_str = format!(
            "{} {} {}\r\n",
            self.version, self.status_code, self.reason_phrase
        );
        for (header_name, header_value) in &self.headers {
            if header_name != &"Content-Length".to_string() {
                response_str += &format!("{header_name}: {header_value}\r\n");
            }
        }

        if let Some(body) = &self.body {
            response_str += &format!("Content-Length: {}\r\n", body.len());
        }

        response_str += "\r\n";
        if let Some(body) = &self.body {
            response_str += body;
        }
        response_str
    }
}

#[cfg(test)]
mod tests {

    use super::Marshal;
    use super::Unmarshal;

    use super::HttpRequest;
    use super::HttpResponse;

    use indexmap::IndexMap;
    use std::io::BufRead;
    #[allow(dead_code)]
    fn read_headers(reader: &mut dyn BufRead) -> Option<IndexMap<String, String>> {
        let mut headers = IndexMap::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(index) = line.find(": ") {
                        let name = line[..index].to_string();
                        let value = line[index + 2..].trim().to_string();
                        headers.insert(name, value);
                    }
                }
                Err(_) => return None,
            }
        }
        Some(headers)
    }

    #[test]
    fn test_parse_http_request() {
        let request = "POST /whip/endpoint?app=live&stream=test HTTP/1.1\r\n\
        Host: whip.example.com\r\n\
        Content-Type: application/sdp\r\n\
        Content-Length: 1326\r\n\
        \r\n\
        v=0\r\n\
        o=- 5228595038118931041 2 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=msid-semantic: WMS\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:EsAw\r\n\
        a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 DA:7B:57:DC:28:CE:04:4F:31:79:85:C4:31:67:EB:27:58:29:ED:77:2A:0D:24:AE:ED:AD:30:BC:BD:F1:9C:02\r\n\
        a=setup:actpass\r\n\
        a=mid:0\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=sendonly\r\n\
        a=msid:- d46fb922-d52a-4e9c-aa87-444eadc1521b\r\n\
        a=rtcp-mux\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:EsAw\r\n\
        a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 DA:7B:57:DC:28:CE:04:4F:31:79:85:C4:31:67:EB:27:58:29:ED:77:2A:0D:24:AE:ED:AD:30:BC:BD:F1:9C:02\r\n\
        a=setup:actpass\r\n\
        a=mid:1\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=sendonly\r\n\
        a=msid:- d46fb922-d52a-4e9c-aa87-444eadc1521b\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 VP8/90000\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n";

        if let Some(parser) = HttpRequest::unmarshal(request) {
            println!(" parser: {parser:?}");
            let marshal_result = parser.marshal();
            print!("marshal result: =={marshal_result}==");
            assert_eq!(request, marshal_result);
        }
    }

    #[test]
    fn test_parse_http_response() {
        let response = "HTTP/1.1 201 Created\r\n\
        Content-Type: application/sdp\r\n\
        Location: https://whip.example.com/resource/id\r\n\
        Content-Length: 1392\r\n\
        \r\n\
        v=0\r\n\
        o=- 1657793490019 1 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=ice-lite\r\n\
        a=msid-semantic: WMS *\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:38sdf4fdsf54\r\n\
        a=ice-pwd:2e13dde17c1cb009202f627fab90cbec358d766d049c9697\r\n\
        a=fingerprint:sha-256 F7:EB:F3:3E:AC:D2:EA:A7:C1:EC:79:D9:B3:8A:35:DA:70:86:4F:46:D9:2D:CC:D0:BC:81:9F:67:EF:34:2E:BD\r\n\
        a=candidate:1 1 UDP 2130706431 198.51.100.1 39132 typ host\r\n\
        a=setup:passive\r\n\
        a=mid:0\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:38sdf4fdsf54\r\n\
        a=ice-pwd:2e13dde17c1cb009202f627fab90cbec358d766d049c9697\r\n\
        a=fingerprint:sha-256 F7:EB:F3:3E:AC:D2:EA:A7:C1:EC:79:D9:B3:8A:35:DA:70:86:4F:46:D9:2D:CC:D0:BC:81:9F:67:EF:34:2E:BD\r\n\
        a=candidate:1 1 UDP 2130706431 198.51.100.1 39132 typ host\r\n\
        a=setup:passive\r\n\
        a=mid:1\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 VP8/90000\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n";

        if let Some(parser) = HttpResponse::unmarshal(response) {
            println!(" parser: {parser:?}");
            let marshal_result = parser.marshal();
            print!("marshal result: =={marshal_result}==");
            assert_eq!(response, marshal_result);
        }
    }
}
