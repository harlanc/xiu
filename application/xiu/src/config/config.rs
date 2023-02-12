use super::errors::ConfigError;
use serde_derive::Deserialize;
use std::fs;
use std::vec::Vec;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rtmp: Option<RtmpConfig>,
    pub httpflv: Option<HttpFlvConfig>,
    pub hls: Option<HlsConfig>,
    pub log: Option<LogConfig>,
}

impl Config {
    pub fn new(rtmp_port: usize, httpflv_port: usize, hls_port: usize, log_level: String) -> Self {
        let mut rtmp_config: Option<RtmpConfig> = None;
        if rtmp_port > 0 {
            rtmp_config = Some(RtmpConfig {
                enabled: true,
                port: rtmp_port,
                pull: None,
                push: None,
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
            });
        }

        let log_config = Some(LogConfig {
            level: log_level,
            file: None,
        });

        Self {
            rtmp: rtmp_config,
            httpflv: httpflv_config,
            hls: hls_config,
            log: log_config,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RtmpConfig {
    pub enabled: bool,
    pub port: usize,
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
pub struct HttpFlvConfig {
    pub enabled: bool,
    pub port: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HlsConfig {
    pub enabled: bool,
    pub port: usize,
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

    let str = fs::read_to_string("/Users/zexu/github/xiu/application/src/config/config.toml");

    match str {
        Ok(val) => {
            println!("++++++{val}\n");
            let decoded: Config = toml::from_str(&val[..]).unwrap();

            let rtmp = decoded.rtmp;
            match rtmp {
                Some(val) => {
                    println!("++++++{}\n", val.enabled);
                }
                None => {}
            }
        }
        Err(err) => println!("======{err}"),
    }
}
