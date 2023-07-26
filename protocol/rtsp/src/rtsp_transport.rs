use crate::global_trait::Marshal;

use super::global_trait::Unmarshal;
use super::rtsp_utils::scanf;

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
    pub cast_type: CastType,
    pub protocol_type: ProtocolType,
    pub interleaved: Option<[u8; 2]>,
    pub transport_mod: Option<String>,
    pub client_port: Option<[u16; 2]>,
    pub server_port: Option<[u16; 2]>,
    pub ssrc: Option<u32>,
}

impl Unmarshal for RtspTransport {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtsp_transport = RtspTransport::default();

        let param_parts: Vec<&str> = raw_data.split(';').collect();
        for part in param_parts {
            let kv: Vec<&str> = part.split('=').collect();
            match kv[0] {
                "RTP/AVP/TCP" => {
                    rtsp_transport.protocol_type = ProtocolType::TCP;
                }
                "RTP/AVP/UDP" | "RTP/AVP" => {
                    rtsp_transport.protocol_type = ProtocolType::UDP;
                }
                "unicast" => {
                    rtsp_transport.cast_type = CastType::Unicast;
                }
                "multicast" => {
                    rtsp_transport.cast_type = CastType::Multicast;
                }
                "mode" => {
                    rtsp_transport.transport_mod = Some(kv[1].to_string());
                }
                "client_port" => {
                    let ports = scanf!(kv[1], '-', u16, u16);

                    let mut client_ports: [u16; 2] = [0, 0];
                    if let Some(port) = ports.0 {
                        client_ports[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        client_ports[1] = port;
                    }

                    rtsp_transport.client_port = Some(client_ports);
                }
                "server_port" => {
                    let ports = scanf!(kv[1], '-', u16, u16);

                    let mut server_ports: [u16; 2] = [0, 0];
                    if let Some(port) = ports.0 {
                        server_ports[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        server_ports[1] = port;
                    }

                    rtsp_transport.server_port = Some(server_ports);
                }
                "interleaved" => {
                    let vals = scanf!(kv[1], '-', u8, u8);

                    let mut interleaveds: [u8; 2] = [0, 0];
                    if let Some(val) = vals.0 {
                        interleaveds[0] = val;
                    }
                    if let Some(val) = vals.1 {
                        interleaveds[1] = val;
                    }

                    rtsp_transport.interleaved = Some(interleaveds);
                }
                "ssrc" => {
                    if let Ok(ssrc) = kv[1].parse::<u32>() {
                        rtsp_transport.ssrc = Some(ssrc);
                    }
                }

                _ => {}
            }
        }

        Some(rtsp_transport)
    }
}

impl Marshal for RtspTransport {
    fn marshal(&self) -> String {
        let protocol_type = match self.protocol_type {
            ProtocolType::TCP => "RTP/AVP/TCP",
            ProtocolType::UDP => "RTP/AVP/UDP",
        };

        let cast_type = match self.cast_type {
            CastType::Multicast => "multicast",
            CastType::Unicast => "unicast",
        };

        let client_port = if let Some(client_ports) = self.client_port {
            format!("client_port={}-{};", client_ports[0], client_ports[1])
        } else {
            String::from("")
        };

        let server_port = if let Some(server_ports) = self.server_port {
            format!("server_port={}-{};", server_ports[0], server_ports[1])
        } else {
            String::from("")
        };

        let interleaved = if let Some(interleaveds) = self.interleaved {
            format!("interleaved={}-{};", interleaveds[0], interleaveds[1])
        } else {
            String::from("")
        };

        let ssrc = if let Some(ssrc) = self.ssrc {
            format!("ssrc={ssrc};")
        } else {
            String::from("")
        };

        let mode = if let Some(mode) = &self.transport_mod {
            format!("mode={mode}")
        } else {
            String::from("")
        };

        format!("{protocol_type};{cast_type};{client_port}{server_port}{interleaved}{ssrc}{mode}")
    }
}

#[cfg(test)]
mod tests {

    use crate::global_trait::Marshal;
    use crate::global_trait::Unmarshal;

    use super::RtspTransport;

    #[test]
    fn test_parse_transport() {
        let parser = RtspTransport::unmarshal(
            "RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record",
        ).unwrap();

        println!(" parser: {parser:?}");

        // assert_eq!(parser.cast_type, CastType::Unicast);
        // assert_eq!(parser.protocol_type, ProtocolType::UDP);
        // assert_eq!(parser.interleaved.unwrap(), [0, 1]);
        // assert_eq!(parser.transport_mod.unwrap(), "record".to_string());
        // assert_eq!(parser.client_port.unwrap(), [8000, 8001]);
        // assert_eq!(parser.server_port.unwrap(), [9000, 9001]);
        // assert_eq!(parser.ssrc.unwrap(), 1234);

        println!("marshal reslut: {}", parser.marshal());
    }
}
