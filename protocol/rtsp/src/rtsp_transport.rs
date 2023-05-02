#[derive(Debug, Clone, Default, PartialEq)]

pub enum CastType {
    Multicast,
    #[default]
    Unicast,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ProtocolType {
    #[default]
    TCP,
    UDP,
}
#[derive(Debug, Clone, Default)]
pub struct RtspTransport {
    cast_type: CastType,
    protocol_type: ProtocolType,
    interleaved: [usize; 2],
    transport_mod: String,
    client_port: [usize; 2],
    server_port: [usize; 2],
    ssrc: u32,
}

macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

impl RtspTransport {
    pub fn parse(&mut self, raw_data: String) {
        let param_parts: Vec<&str> = raw_data.split(';').collect();
        for part in param_parts {
            let kv: Vec<&str> = part.split('=').collect();
            match kv[0] {
                "RTP/AVP/TCP" => {
                    self.protocol_type = ProtocolType::TCP;
                }
                "RTP/AVP/UDP" | "RTP/AVP" => {
                    self.protocol_type = ProtocolType::UDP;
                }
                "unicast" => {
                    self.cast_type = CastType::Unicast;
                }
                "multicast" => {
                    self.cast_type = CastType::Multicast;
                }
                "mode" => {
                    self.transport_mod = kv[1].to_string();
                }
                "client_port" => {
                    let ports = scanf!(kv[1], '-', usize, usize);
                    if let Some(port) = ports.0 {
                        self.client_port[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        self.client_port[1] = port;
                    }
                }
                "server_port" => {
                    let ports = scanf!(kv[1], '-', usize, usize);
                    if let Some(port) = ports.0 {
                        self.server_port[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        self.server_port[1] = port;
                    }
                }
                "interleaved" => {
                    let vals = scanf!(kv[1], '-', usize, usize);
                    if let Some(val) = vals.0 {
                        self.interleaved[0] = val;
                    }
                    if let Some(val) = vals.1 {
                        self.interleaved[1] = val;
                    }
                }
                "ssrc" => {
                    if let Ok(ssrc) = kv[1].parse::<u32>() {
                        self.ssrc = ssrc;
                    }
                }

                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::CastType;
    use super::ProtocolType;
    use super::RtspTransport;

    #[test]
    fn test_parse_transport() {
        let mut parser = RtspTransport::default();

        parser.parse(
            "RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record".to_string(),
        );

        println!(" parser: {:?}", parser);

        assert_eq!(parser.cast_type, CastType::Unicast);
        assert_eq!(parser.protocol_type, ProtocolType::UDP);
        assert_eq!(parser.interleaved, [0, 1]);
        assert_eq!(parser.transport_mod, "record".to_string());
        assert_eq!(parser.client_port, [8000, 8001]);
        assert_eq!(parser.server_port, [9000, 9001]);
        assert_eq!(parser.ssrc, 1234);
    }
}
