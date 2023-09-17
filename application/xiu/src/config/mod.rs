pub mod errors;

use errors::ConfigError;
use serde_derive::Deserialize;
use std::fs;
use std::vec::Vec;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rtmp: Option<RtmpConfig>,
    pub rtsp: Option<RtspConfig>,
    pub webrtc: Option<WebRTCConfig>,
    pub httpflv: Option<HttpFlvConfig>,
    pub hls: Option<HlsConfig>,
    pub httpapi: Option<HttpApiConfig>,
    pub httpnotify: Option<HttpNotifierConfig>,
    pub log: Option<LogConfig>,
}

impl Config {
    pub fn new(
        rtmp_port: usize,
        rtsp_port: usize,
        webrtc_port: usize,
        httpflv_port: usize,
        hls_port: usize,
        log_level: String,
    ) -> Self {
        let mut rtmp_config: Option<RtmpConfig> = None;
        if rtmp_port > 0 {
            rtmp_config = Some(RtmpConfig {
                enabled: true,
                gop_num: Some(1),
                port: rtmp_port,
                pull: None,
                push: None,
            });
        }

        let mut rtsp_config: Option<RtspConfig> = None;
        if rtsp_port > 0 {
            rtsp_config = Some(RtspConfig {
                enabled: true,
                port: rtsp_port,
            });
        }

        let mut webrtc_config: Option<WebRTCConfig> = None;
        if webrtc_port > 0 {
            webrtc_config = Some(WebRTCConfig {
                enabled: true,
                port: webrtc_port,
            });
        }

        let mut httpflv_config: Option<HttpFlvConfig> = None;
        if httpflv_port > 0 {
            httpflv_config = Some(HttpFlvConfig {
                enabled: true,
                port: httpflv_port,
            });
        }

        let mut hls_config: Option<HlsConfig> = None;
        if hls_port > 0 {
            hls_config = Some(HlsConfig {
                enabled: true,
                port: hls_port,
                need_record: false,
            });
        }

        let log_config = Some(LogConfig {
            level: log_level,
            file: None,
        });

        Self {
            rtmp: rtmp_config,
            rtsp: rtsp_config,
            webrtc: webrtc_config,
            httpflv: httpflv_config,
            hls: hls_config,
            httpapi: None,
            httpnotify: None,
            log: log_config,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RtmpConfig {
    pub enabled: bool,
    pub port: usize,
    pub gop_num: Option<usize>,
    pub pull: Option<RtmpPullConfig>,
    pub push: Option<Vec<RtmpPushConfig>>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RtmpPullConfig {
    pub enabled: bool,
    pub address: String,
    pub port: u16,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RtmpPushConfig {
    pub enabled: bool,
    pub address: String,
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RtspConfig {
    pub enabled: bool,
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebRTCConfig {
    pub enabled: bool,
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpFlvConfig {
    pub enabled: bool,
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HlsConfig {
    pub enabled: bool,
    pub port: usize,
    //record or not
    pub need_record: bool,
}

pub enum LogLevel {
    Info,
    Warn,
    Error,
    Trace,
    Debug,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<LogFile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogFile {
    pub enabled: bool,
    pub rotate: String,
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpApiConfig {
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpNotifierConfig {
    pub enabled: bool,
    pub on_publish: Option<String>,
    pub on_unpublish: Option<String>,
    pub on_play: Option<String>,
    pub on_stop: Option<String>,
}

pub fn load(cfg_path: &String) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(cfg_path)?;
    let decoded_config = toml::from_str(&content[..]).unwrap();
    Ok(decoded_config)
}

#[test]
fn test_toml_parse() {
    // let path = env::current_dir();
    // match path {
    //     Ok(val) => println!("The current directory is {}\n", val.display()),
    //     Err(err) => print!("{}\n", err),
    // }

    let str = fs::read_to_string(
        "/Users/zexu/github/xiu_live_rust/application/xiu/src/config/config.toml",
    );

    match str {
        Ok(val) => {
            println!("++++++{val}\n");
            let decoded: Config = toml::from_str(&val[..]).unwrap();

            let rtmp = decoded.httpnotify;

            if let Some(val) = rtmp {
                println!("++++++{val:?}\n");
            }
        }
        Err(err) => println!("======{err}"),
    }
}
