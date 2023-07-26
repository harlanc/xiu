use crate::global_trait::Marshal;
use crate::global_trait::Unmarshal;
use crate::rtsp_utils;
use indexmap::IndexMap;

#[derive(Debug, Clone, Default)]
pub struct RtspRequest {
    pub method: String,
    pub url: String,
    //url = "rtsp://{}:{}/{}", address, port, path
    pub address: String,
    pub port: u16,
    pub path: String,
    pub version: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl RtspRequest {
    pub fn get_header(&self, header_name: &String) -> Option<&String> {
        self.headers.get(header_name)
    }
}

impl Unmarshal for RtspRequest {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut rtsp_request = RtspRequest::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();
                if let Some(method) = fields.next() {
                    rtsp_request.method = method.to_string();
                }
                if let Some(url) = fields.next() {
                    rtsp_request.url = url.to_string();

                    if let Some(val) = url.strip_prefix("rtsp://") {
                        if let Some(index) = val.find('/') {
                            let path = &url[7 + index + 1..];
                            rtsp_request.path = String::from(path);
                            let address_with_port = &url[7..7 + index];

                            let (address_val, port_val) =
                                rtsp_utils::scanf!(address_with_port, ':', String, u16);

                            if let Some(address) = address_val {
                                rtsp_request.address = address;
                            }
                            if let Some(port) = port_val {
                                rtsp_request.port = port;
                            }

                            print!("address_with_port: {address_with_port}");
                        }
                    }
                }
                if let Some(version) = fields.next() {
                    rtsp_request.version = version.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    rtsp_request.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            rtsp_request.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(rtsp_request)
    }
}

impl Marshal for RtspRequest {
    fn marshal(&self) -> String {
        let mut request_str = format!("{} {} {}\r\n", self.method, self.url, self.version);
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
pub struct RtspResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl Unmarshal for RtspResponse {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut rtsp_response = RtspResponse::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();

                if let Some(version) = fields.next() {
                    rtsp_response.version = version.to_string();
                }
                if let Some(status) = fields.next() {
                    if let Ok(status) = status.parse::<u16>() {
                        rtsp_response.status_code = status;
                    }
                }
                if let Some(reason_phrase) = fields.next() {
                    rtsp_response.reason_phrase = reason_phrase.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    rtsp_response.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            rtsp_response.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(rtsp_response)
    }
}

impl Marshal for RtspResponse {
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

    use crate::{global_trait::Marshal, http::Unmarshal};

    use super::RtspRequest;

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

        if let Some(parser) = RtspRequest::unmarshal(data1) {
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

        if let Some(parser) = RtspRequest::unmarshal(data2) {
            println!(" parser: {parser:?}");
            let marshal_result = parser.marshal();
            print!("marshal result: =={marshal_result}==");
            assert_eq!(data2, marshal_result);
        }
    }

    #[test]
    fn test_http_status_code() {
        let stats_code = http::StatusCode::OK;

        println!(
            "stats_code: {}, {}",
            stats_code.canonical_reason().unwrap(),
            stats_code.as_u16()
        )
    }
}
