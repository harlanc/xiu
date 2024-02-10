use crate::scanf;
use indexmap::IndexMap;
use std::fmt;

pub trait Unmarshal {
    fn unmarshal(request_data: &str) -> Option<Self>
    where
        Self: Sized;
}

pub trait Marshal {
    fn marshal(&self) -> String;
}

#[derive(Debug, Clone, Default)]
pub enum Schema {
    //used for webrtc(WHIP/WHEP)
    WEBRTC,
    RTSP,
    #[default]
    UNKNOWN,
}

impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Schema::RTSP => {
                write!(f, "rtsp")
            }
            //Because webrtc request uri does not contain the schema name, so here write empty string.
            Schema::WEBRTC => {
                write!(f, "")
            }
            Schema::UNKNOWN => {
                write!(f, "unknown")
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Uri {
    pub schema: Schema,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
}

impl Unmarshal for Uri {
    /*
    RTSP HTTP header : "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
    the uri rtsp://127.0.0.1:5544/stream is standard with schema://host:port/path?query.

    WEBRTC is special:
    "POST /whep?app=live&stream=test HTTP/1.1\r\n\
     Host: localhost:3000\r\n\
     Accept: \r\n\"
    It only contains path?query after HTTP method, host:port is saved in the Host parameter.
    In this function, for Webrtc we only parse path?query, host:port will be parsed in the HTTPRequest
    unmarshal method.
    */
    fn unmarshal(url: &str) -> Option<Self> {
        let mut uri = Uri::default();

        /*first judge the correct schema */
        if url.starts_with("rtsp://") {
            uri.schema = Schema::RTSP;
        } else if url.starts_with("/whip") || url.starts_with("/whep") {
            uri.schema = Schema::WEBRTC;
        } else {
            log::warn!("cannot judge the schema: {}", url);
            uri.schema = Schema::UNKNOWN;
        }

        let path_with_query = match uri.schema {
            Schema::RTSP => {
                let rtsp_path_with_query = if let Some(rtsp_url_without_prefix) =
                    url.strip_prefix("rtsp://")
                {
                    /*split host:port and path?query*/
                    
                    if let Some(index) = rtsp_url_without_prefix.find('/') {
                        let path_with_query = &rtsp_url_without_prefix[index + 1..];
                        /*parse host and port*/
                        let host_with_port = &rtsp_url_without_prefix[..index];
                        let (host_val, port_val) = scanf!(host_with_port, ':', String, u16);
                        if let Some(host) = host_val {
                            uri.host = host;
                        }
                        if let Some(port) = port_val {
                            uri.port = Some(port);
                        }

                        path_with_query
                    } else {
                        log::error!("cannot find split '/' for host:port and path?query.");
                        return None;
                    }
                } else {
                    log::error!("cannot find RTSP prefix.");
                    return None;
                };
                rtsp_path_with_query
            }
            Schema::WEBRTC => url,
            Schema::UNKNOWN => url,
        };

        let path_data: Vec<&str> = path_with_query.splitn(2, '?').collect();
        uri.path = path_data[0].to_string();

        if path_data.len() > 1 {
            uri.query = Some(path_data[1].to_string());
        }

        Some(uri)
    }
}

impl Marshal for Uri {
    fn marshal(&self) -> String {
        /*first pice path and query together*/
        let path_with_query = if let Some(query) = &self.query {
            format!("{}?{}", self.path, query)
        } else {
            self.path.clone()
        };

        match self.schema {
            Schema::RTSP => {
                let host_with_port = if let Some(port) = &self.port {
                    format!("{}:{}", self.host, port)
                } else {
                    self.host.clone()
                };
                format!("{}://{}/{}", self.schema, host_with_port, path_with_query)
            }
            Schema::WEBRTC => path_with_query,
            Schema::UNKNOWN => path_with_query,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub uri: Uri,
    /*parse the query and save the results*/
    pub query_pairs: IndexMap<String, String>,
    pub version: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn get_header(&self, header_name: &String) -> Option<&String> {
        self.headers.get(header_name)
    }
}

pub fn parse_content_length(request_data: &str) -> Option<u32> {
    let start = "Content-Length:";
    let end = "\r\n";

    let start_index = request_data.find(start)? + start.len();
    let end_index = request_data[start_index..].find(end)? + start_index;
    let length_str = &request_data[start_index..end_index];

    length_str.trim().parse().ok()
}

impl Unmarshal for HttpRequest {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut http_request = HttpRequest::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            /*parse the first line
            POST /whip?app=live&stream=test HTTP/1.1*/
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();
                /* method */
                if let Some(method) = fields.next() {
                    http_request.method = method.to_string();
                }
                /* url */
                if let Some(url) = fields.next() {
                    if let Some(uri) = Uri::unmarshal(url) {
                        http_request.uri = uri;

                        if let Some(query) = &http_request.uri.query {
                            let pars_array: Vec<&str> = query.split('&').collect();

                            for ele in pars_array {
                                let (k, v) = scanf!(ele, '=', String, String);
                                if k.is_none() || v.is_none() {
                                    continue;
                                }
                                http_request.query_pairs.insert(k.unwrap(), v.unwrap());
                            }
                        }
                    } else {
                        log::error!("cannot get a Uri.");
                        return None;
                    }
                }
                /* version */
                if let Some(version) = fields.next() {
                    http_request.version = version.to_string();
                }
            }
            /*parse headers*/
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    /*for schema: webrtc*/
                    if name == "Host" {
                        let (address_val, port_val) = scanf!(value, ':', String, u16);
                        if let Some(address) = address_val {
                            http_request.uri.host = address;
                        }
                        if let Some(port) = port_val {
                            http_request.uri.port = Some(port);
                        }
                    }
                    http_request.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };
        log::trace!(
            "header_end_idx is: {} {}",
            header_end_idx,
            request_data.len()
        );

        if request_data.len() > header_end_idx {
            /*parse body*/
            http_request.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(http_request)
    }
}

impl Marshal for HttpRequest {
    fn marshal(&self) -> String {
        let mut request_str = format!(
            "{} {} {}\r\n",
            self.method,
            self.uri.marshal(),
            self.version
        );
        for (header_name, header_value) in &self.headers {
            if header_name == &"Content-Length".to_string() {
                if let Some(body) = &self.body {
                    request_str += &format!("Content-Length: {}\r\n", body.len());
                }
            } else {
                request_str += &format!("{header_name}: {header_value}\r\n");
            }
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
    fn test_whep_request() {
        let request = "POST /whep?app=live&stream=test HTTP/1.1\r\n\
        Host: localhost:3000\r\n\
        Accept: */*\r\n\
        Sec-Fetch-Site: same-origin\r\n\
        Accept-Language: zh-cn\r\n\
        Accept-Encoding: gzip, deflate\r\n\
        Sec-Fetch-Mode: cors\r\n\
        Content-Type: application/sdp\r\n\
        Origin: http://localhost:3000\r\n\
        User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15\r\n\
        Referer: http://localhost:3000/\r\n\
        Content-Length: 3895\r\n\
        Connection: keep-alive\r\n\
        Sec-Fetch-Dest: empty\r\n\
        \r\n\
        v=0\r\n\
        o=- 6550659986740559335 2 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=msid-semantic: WMS\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97 98 99 100 101 102 125 104 124 106 107 108 109 127 35\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:0mum\r\n\
        a=ice-pwd:DD4LnAhZLQNLSzRZWZRh9Jm4\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 6C:61:89:FF:9D:2F:BA:0A:A4:80:0D:98:C3:CA:43:05:82:EB:59:13:BC:C8:DE:33:2F:26:4A:27:D8:D0:D1:3D\r\n\
        a=setup:actpass\r\n\
        a=mid:0\r\n\
        a=extmap:1 urn:ietf:params:rtp-hdrext:toffset\r\n\
        a=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\n\
        a=extmap:3 urn:3gpp:video-orientation\r\n\
        a=extmap:4 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\n\
        a=extmap:5 http://www.webrtc.org/experiments/rtp-hdrext/playout-delay\r\n\
        a=extmap:6 http://www.webrtc.org/experiments/rtp-hdrext/video-content-type\r\n\
        a=extmap:7 http://www.webrtc.org/experiments/rtp-hdrext/video-timing\r\n\
        a=extmap:8 http://www.webrtc.org/experiments/rtp-hdrext/color-space\r\n\
        a=extmap:9 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=rtcp-fb:96 goog-remb\r\n\
        a=rtcp-fb:96 transport-cc\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=640c1f\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n\
        a=rtpmap:98 H264/90000\r\n\
        a=rtcp-fb:98 goog-remb\r\n\
        a=rtcp-fb:98 transport-cc\r\n\
        a=rtcp-fb:98 ccm fir\r\n\
        a=rtcp-fb:98 nack\r\n\
        a=rtcp-fb:98 nack pli\r\n\
        a=fmtp:98 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r\n\
        a=rtpmap:99 rtx/90000\r\n\
        a=fmtp:99 apt=98\r\n\
        a=rtpmap:100 H264/90000\r\n\
        a=rtcp-fb:100 goog-remb\r\n\
        a=rtcp-fb:100 transport-cc\r\n\
        a=rtcp-fb:100 ccm fir\r\n\
        a=rtcp-fb:100 nack\r\n\
        a=rtcp-fb:100 nack pli\r\n\
        a=fmtp:100 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=640c1f\r\n\
        a=rtpmap:101 rtx/90000\r\n\
        a=fmtp:101 apt=100\r\n\
        a=rtpmap:102 H264/90000\r\n\
        a=rtcp-fb:102 goog-remb\r\n\
        a=rtcp-fb:102 transport-cc\r\n\
        a=rtcp-fb:102 ccm fir\r\n\
        a=rtcp-fb:102 nack\r\n\
        a=rtcp-fb:102 nack pli\r\n\
        a=fmtp:102 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42e01f\r\n\
        a=rtpmap:125 rtx/90000\r\n\
        a=fmtp:125 apt=102\r\n\
        a=rtpmap:104 VP8/90000\r\n\
        a=rtcp-fb:104 goog-remb\r\n\
        a=rtcp-fb:104 transport-cc\r\n\
        a=rtcp-fb:104 ccm fir\r\n\
        a=rtcp-fb:104 nack\r\n\
        a=rtcp-fb:104 nack pli\r\n\
        a=rtpmap:124 rtx/90000\r\n\
        a=fmtp:124 apt=104\r\n\
        a=rtpmap:106 VP9/90000\r\n\
        a=rtcp-fb:106 goog-remb\r\n\
        a=rtcp-fb:106 transport-cc\r\n\
        a=rtcp-fb:106 ccm fir\r\n\
        a=rtcp-fb:106 nack\r\n\
        a=rtcp-fb:106 nack pli\r\n\
        a=fmtp:106 profile-id=0\r\n\
        a=rtpmap:107 rtx/90000\r\n\
        a=fmtp:107 apt=106\r\n\
        a=rtpmap:108 red/90000\r\n\
        a=rtpmap:109 rtx/90000\r\n\
        a=fmtp:109 apt=108\r\n\
        a=rtpmap:127 ulpfec/90000\r\n\
        a=rtpmap:35 flexfec-03/90000\r\n\
        a=rtcp-fb:35 goog-remb\r\n\
        a=rtcp-fb:35 transport-cc\r\n\
        a=fmtp:35 repair-window=10000000\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111 63 103 9 0 8 105 13 110 113 126\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:0mum\r\n\
        a=ice-pwd:DD4LnAhZLQNLSzRZWZRh9Jm4\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 6C:61:89:FF:9D:2F:BA:0A:A4:80:0D:98:C3:CA:43:05:82:EB:59:13:BC:C8:DE:33:2F:26:4A:27:D8:D0:D1:3D\r\n\
        a=setup:actpass\r\n\
        a=mid:1\r\n\
        a=extmap:14 urn:ietf:params:rtp-hdrext:ssrc-audio-level\r\n\
        a=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\n\
        a=extmap:4 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\n\
        a=extmap:9 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=rtcp-fb:111 transport-cc\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        a=rtpmap:63 red/48000/2\r\n\
        a=fmtp:63 111/111\r\n\
        a=rtpmap:103 ISAC/16000\r\n\
        a=rtpmap:9 G722/8000\r\n\
        a=rtpmap:0 PCMU/8000\r\n\
        a=rtpmap:8 PCMA/8000\r\n\
        a=rtpmap:105 CN/16000\r\n\
        a=rtpmap:13 CN/8000\r\n\
        a=rtpmap:110 telephone-event/48000\r\n\
        a=rtpmap:113 telephone-event/16000\r\n\
        a=rtpmap:126 telephone-event/8000\r\n";

        if let Some(l) = super::parse_content_length(request) {
            println!("content length is : {l}");
        }

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

    // #[test]
    // fn test_parse_rtsp_request_chatgpt() {
    //     let data1 = "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
    //     Content-Type: application/sdp\r\n\
    //     CSeq: 2\r\n\
    //     User-Agent: Lavf58.76.100\r\n\
    //     Content-Length: 500\r\n\
    //     \r\n\
    //     v=0\r\n\
    //     o=- 0 0 IN IP4 127.0.0.1\r\n\
    //     s=No Name\r\n\
    //     c=IN IP4 127.0.0.1\r\n\
    //     t=0 0\r\n\
    //     a=tool:libavformat 58.76.100\r\n\
    //     m=video 0 RTP/AVP 96\r\n\
    //     b=AS:284\r\n\
    //     a=rtpmap:96 H264/90000
    //     a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
    //     a=control:streamid=0\r\n\
    //     m=audio 0 RTP/AVP 97\r\n\
    //     b=AS:128\r\n\
    //     a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
    //     a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
    //     a=control:streamid=1\r\n";

    //     // v=0：SDP版本号，通常为0。
    //     // o=- 0 0 IN IP4 127.0.0.1：会话的所有者和会话ID，以及会话开始时间和会话结束时间的信息。
    //     // s=No Name：会话名称或标题。
    //     // c=IN IP4 127.0.0.1：表示会话数据传输的地址类型(IPv4)和地址(127.0.0.1)。
    //     // t=0 0：会话时间，包括会话开始时间和结束时间，这里的值都是0，表示会话没有预定义的结束时间。
    //     // a=tool:libavformat 58.76.100：会话所使用的工具或软件名称和版本号。

    //     // m=video 0 RTP/AVP 96：媒体类型(video或audio)、媒体格式(RTP/AVP)、媒体格式编号(96)和媒体流的传输地址。
    //     // b=AS:284：视频流所使用的带宽大小。
    //     // a=rtpmap:96 H264/90000：视频流所使用的编码方式(H.264)和时钟频率(90000)。
    //     // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E：视频流的格式参数，如分片方式、SPS和PPS等。
    //     // a=control:streamid=0：指定视频流的流ID。

    //     // m=audio 0 RTP/AVP 97：媒体类型(audio)、媒体格式(RTP/AVP)、媒体格式编号(97)和媒体流的传输地址。
    //     // b=AS:128：音频流所使用的带宽大小。
    //     // a=rtpmap:97 MPEG4-GENERIC/48000/2：音频流所使用的编码方式(MPEG4-GENERIC)、采样率(48000Hz)、和通道数(2)。
    //     // a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500：音频流的格式参数，如编码方式、采样长度、索引长度等。
    //     // a=control:streamid=1：指定音频流的流ID。

    //     if let Some(request) = parse_request_bytes(&data1.as_bytes()) {
    //         println!(" parser: {:?}", request);
    //     }
    // }

    #[test]
    fn test_parse_rtsp_request() {
        let data1 = "SETUP rtsp://127.0.0.1/stream/streamid=0 RTSP/1.0\r\n\
        Transport: RTP/AVP/TCP;unicast;interleaved=0-1;mode=record\r\n\
        CSeq: 3\r\n\
        User-Agent: Lavf58.76.100\r\n\
        \r\n";

        if let Some(parser) = HttpRequest::unmarshal(data1) {
            println!(" parser: {parser:?}");
            let marshal_result = parser.marshal();
            print!("marshal result: =={marshal_result}==");
            assert_eq!(data1, marshal_result);
        }

        let data2 = "ANNOUNCE rtsp://127.0.0.1/stream RTSP/1.0\r\n\
        Content-Type: application/sdp\r\n\
        CSeq: 2\r\n\
        User-Agent: Lavf58.76.100\r\n\
        Content-Length: 500\r\n\
        \r\n\
        v=0\r\n\
        o=- 0 0 IN IP4 127.0.0.1\r\n\
        s=No Name\r\n\
        c=IN IP4 127.0.0.1\r\n\
        t=0 0\r\n\
        a=tool:libavformat 58.76.100\r\n\
        m=video 0 RTP/AVP 96\r\n\
        b=AS:284\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
        a=control:streamid=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        b=AS:128\r\n\
        a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
        a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
        a=control:streamid=1\r\n";

        if let Some(parser) = HttpRequest::unmarshal(data2) {
            println!(" parser: {parser:?}");
            let marshal_result = parser.marshal();
            print!("marshal result: =={marshal_result}==");
            assert_eq!(data2, marshal_result);
        }
    }
}
