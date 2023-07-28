use crate::global_trait::{Marshal, Unmarshal};
use base64::{engine::general_purpose, Engine as _};
use bytes::{BufMut, BytesMut};

// pub trait Fmtp: TMsgConverter {}

#[derive(Debug, Clone, Default)]
pub struct H264Fmtp {
    pub payload_type: u16,
    packetization_mode: u8,
    profile_level_id: BytesMut,
    pub sps: BytesMut,
    pub pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
pub struct H265Fmtp {
    pub payload_type: u16,
    pub vps: BytesMut,
    pub sps: BytesMut,
    pub pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
pub struct Mpeg4Fmtp {
    pub payload_type: u16,
    pub asc: BytesMut,
    profile_level_id: BytesMut,
    mode: String,
    size_length: u16,
    index_length: u16,
    index_delta_length: u16,
}
#[derive(Debug, Clone)]
pub enum Fmtp {
    H264(H264Fmtp),
    H265(H265Fmtp),
    Mpeg4(Mpeg4Fmtp),
}

impl Fmtp {
    pub fn new(codec: &str, raw_data: &str) -> Option<Fmtp> {
        match codec.to_lowercase().as_str() {
            "h264" => {
                if let Some(h264_fmtp) = H264Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::H264(h264_fmtp));
                }
            }
            "h265" => {
                if let Some(h265_fmtp) = H265Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::H265(h265_fmtp));
                }
            }
            "mpeg4-generic" => {
                if let Some(mpeg4_fmtp) = Mpeg4Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::Mpeg4(mpeg4_fmtp));
                }
            }
            _ => {}
        }
        None
    }

    pub fn marshal(&self) -> String {
        match self {
            Fmtp::H264(h264fmtp) => h264fmtp.marshal(),
            Fmtp::H265(h265fmtp) => h265fmtp.marshal(),
            Fmtp::Mpeg4(mpeg4fmtp) => mpeg4fmtp.marshal(),
        }
    }
}

// a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
impl Unmarshal for H264Fmtp {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut h264_fmtp = H264Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H264FmtpSdp parse err: {}", raw_data);
            return None;
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            h264_fmtp.payload_type = payload_type;
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H264FmtpSdp parse key=value err: {}", parameter);
                continue;
            }
            match kv[0] {
                "packetization-mode" => {
                    if let Ok(packetization_mode) = kv[1].parse::<u8>() {
                        h264_fmtp.packetization_mode = packetization_mode;
                    }
                }
                "sprop-parameter-sets" => {
                    let spspps: Vec<&str> = kv[1].split(',').collect();
                    let sps = general_purpose::STANDARD.decode(spspps[0]).unwrap();
                    h264_fmtp.sps.put(&sps[..]);
                    let pps = general_purpose::STANDARD.decode(spspps[1]).unwrap();
                    h264_fmtp.pps.put(&pps[..]);
                }
                "profile-level-id" => {
                    h264_fmtp.profile_level_id = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(h264_fmtp)
    }
}

impl Marshal for H264Fmtp {
    // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
    fn marshal(&self) -> String {
        let sps_str = general_purpose::STANDARD.encode(&self.sps);
        let pps_str = general_purpose::STANDARD.encode(&self.pps);
        let profile_level_id_str = String::from_utf8(self.profile_level_id.to_vec()).unwrap();

        let h264_fmtp = format!(
            "{} packetization-mode={}; sprop-parameter-sets={},{}; profile-level-id={}",
            self.payload_type, self.packetization_mode, sps_str, pps_str, profile_level_id_str
        );

        format!("{h264_fmtp}\r\n")
    }
}

impl Unmarshal for H265Fmtp {
    //"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ"
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut h265_fmtp = H265Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H265FmtpSdp parse err: {}", raw_data);
            return None;
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            h265_fmtp.payload_type = payload_type;
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H265FmtpSdp parse key=value err: {}", parameter);
                continue;
            }

            match kv[0] {
                "sprop-vps" => {
                    h265_fmtp.vps = kv[1].into();
                }
                "sprop-sps" => {
                    h265_fmtp.sps = kv[1].into();
                }
                "sprop-pps" => {
                    h265_fmtp.pps = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(h265_fmtp)
    }
}

impl Marshal for H265Fmtp {
    //"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ"
    fn marshal(&self) -> String {
        let sps_str = String::from_utf8(self.sps.to_vec()).unwrap();
        let pps_str = String::from_utf8(self.pps.to_vec()).unwrap();
        let vps_str = String::from_utf8(self.vps.to_vec()).unwrap();

        let h265_fmtp = format!(
            "{} sprop-vps={}; sprop-sps={}; sprop-pps={}",
            self.payload_type, vps_str, sps_str, pps_str
        );

        format!("{h265_fmtp}\r\n")
    }
}

impl Unmarshal for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut mpeg4_fmtp = Mpeg4Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("Mpeg4FmtpSdp parse err: {}", raw_data);
            return None;
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            mpeg4_fmtp.payload_type = payload_type;
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("Mpeg4FmtpSdp parse key=value err: {}", parameter);
                continue;
            }
            match kv[0].to_lowercase().as_str() {
                "mode" => {
                    mpeg4_fmtp.mode = kv[1].to_string();
                }
                "config" => {
                    let asc = hex::decode(kv[1]).unwrap();
                    mpeg4_fmtp.asc.put(&asc[..]);
                }
                "profile-level-id" => {
                    mpeg4_fmtp.profile_level_id = kv[1].into();
                }
                "sizelength" => {
                    if let Ok(size_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.size_length = size_length;
                    }
                }
                "indexlength" => {
                    if let Ok(index_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.index_length = index_length;
                    }
                }
                "indexdeltalength" => {
                    if let Ok(index_delta_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.index_delta_length = index_delta_length;
                    }
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(mpeg4_fmtp)
    }
}

impl Marshal for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn marshal(&self) -> String {
        let profile_level_id_str = String::from_utf8(self.profile_level_id.to_vec()).unwrap();
        let asc_str = hex::encode(&self.asc); //String::from_utf8(self.asc.to_vec()).unwrap();

        let mpeg4_fmtp = format!(
            "{} profile-level-id={};mode={};sizelength={};indexlength={};indexdeltalength={}; config={}",
            self.payload_type, profile_level_id_str, self.mode, self.size_length, self.index_length,
            self.index_delta_length,asc_str);

        format!("{mpeg4_fmtp}\r\n")
    }
}

#[cfg(test)]
mod tests {

    use bytes::BytesMut;

    use super::H264Fmtp;
    use super::H265Fmtp;
    use super::Mpeg4Fmtp;
    use crate::global_trait::Marshal;
    use crate::global_trait::Unmarshal;
    use crate::rtsp_utils;

    #[test]
    fn test_parse_h264fmtpsdp() {
        let parser =  H264Fmtp::unmarshal("96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016").unwrap();

        println!(" parser: {parser:?}");

        assert_eq!(parser.packetization_mode, 1);
        assert_eq!(parser.profile_level_id, "640016");
        // assert_eq!(parser.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        // assert_eq!(parser.pps, "aOvDyyLA");
        //"96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016"

        print!("264 parser: {}", parser.marshal());

        let parser2 = H264Fmtp::unmarshal("96 packetization-mode=1;\nsprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA;\nprofile-level-id=640016").unwrap();

        println!(" parser: {parser2:?}");

        assert_eq!(parser2.packetization_mode, 1);
        assert_eq!(parser2.profile_level_id, "640016");
        // assert_eq!(parser2.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        // assert_eq!(parser2.pps, "aOvDyyLA");

        print!("264 parser2: {}", parser2.marshal());
    }
    #[test]
    fn test_parse_h265fmtpsdp() {
        let parser = H265Fmtp::unmarshal("96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ").unwrap();

        println!(" parser: {parser:?}");

        assert_eq!(parser.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
        assert_eq!(
            parser.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
        assert_eq!(parser.pps, "RAHAc8GJ");

        print!("265 parser: {}", parser.marshal());
    }

    #[test]
    fn test_parse_mpeg4fmtpsdp() {
        let parser = Mpeg4Fmtp::unmarshal("97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500").unwrap();

        println!(" parser: {parser:?}");

        assert_eq!(parser.asc, "121056e500");
        assert_eq!(parser.profile_level_id, "1");
        assert_eq!(parser.mode, "AAC-hbr");
        assert_eq!(parser.size_length, 13);
        assert_eq!(parser.index_length, 3);
        assert_eq!(parser.index_delta_length, 23);

        print!("mpeg4 parser: {}", parser.marshal());
    }

    #[test]
    fn test_string() {
        let s = String::from("119056E500");
        let ss = hex::decode(s).unwrap();
        let b = BytesMut::from(&ss[..]);

        rtsp_utils::print("test", b);
    }
}
