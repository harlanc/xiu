pub mod errors;

use commonlib::auth::AuthAlgorithm;
use errors::{ConfigError, ConfigErrorValue};
use hls::config::{HlsRemuxerConfig, S3UploadConfig};
use serde_derive::Deserialize;
use std::{fs, vec::Vec};
use streamhub::notify::HttpNotifierConfig as StreamhubHttpNotifierConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rtmp: Option<RtmpConfig>,
    pub rtsp: Option<RtspConfig>,
    pub webrtc: Option<WebRTCConfig>,
    pub httpflv: Option<HttpFlvConfig>,
    pub hls: Option<HlsConfig>,
    pub httpapi: Option<HttpApiConfig>,
    pub httpnotify: Option<HttpNotifierConfig>,
    pub authsecret: AuthSecretConfig,
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
        let rtmp = (rtmp_port > 0).then_some(RtmpConfig {
            enabled: true,
            gop_num: Some(1),
            gop_max_frame_num: None,
            port: rtmp_port,
            pull: None,
            push: None,
            auth: None,
        });
        let rtsp = (rtsp_port > 0).then_some(RtspConfig {
            enabled: true,
            relay_enabled: false,
            port: rtsp_port,
            auth: None,
        });
        let webrtc = (webrtc_port > 0).then_some(WebRTCConfig {
            enabled: true,
            port: webrtc_port,
            auth: None,
        });
        let httpflv = (httpflv_port > 0).then_some(HttpFlvConfig {
            enabled: true,
            port: httpflv_port,
            auth: None,
        });
        let hls = (hls_port > 0).then_some(HlsConfig {
            enabled: true,
            port: hls_port,
            need_record: false,
            auth: None,
            path: None,
            fragment: None,
            aof_ratio: None,
            live_ts_count: None,
            prefix: None,
            s3: None,
        });
        Self {
            rtmp,
            rtsp,
            webrtc,
            httpflv,
            hls,
            httpapi: None,
            httpnotify: None,
            authsecret: AuthSecretConfig::default(),
            log: Some(LogConfig {
                level: log_level,
                file: None,
            }),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RtmpConfig {
    pub enabled: bool,
    pub port: usize,
    pub gop_num: Option<usize>,
    pub gop_max_frame_num: Option<usize>,
    pub pull: Option<RtmpPullConfig>,
    pub push: Option<Vec<RtmpPushConfig>>,
    pub auth: Option<AuthConfig>,
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
    pub auth: Option<AuthConfig>,
    pub relay_enabled: bool,
}
#[derive(Debug, Deserialize, Clone)]
pub struct WebRTCConfig {
    pub enabled: bool,
    pub port: usize,
    pub auth: Option<AuthConfig>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct HttpFlvConfig {
    pub enabled: bool,
    pub port: usize,
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HlsConfig {
    pub enabled: bool,
    pub port: usize,
    pub need_record: bool,
    pub auth: Option<AuthConfig>,
    pub path: Option<String>,
    pub fragment: Option<i64>,
    pub aof_ratio: Option<i64>,
    pub live_ts_count: Option<usize>,
    pub prefix: Option<String>,
    pub s3: Option<S3Config>,
}

impl HlsConfig {
    pub fn remuxer_config(&self) -> HlsRemuxerConfig {
        HlsRemuxerConfig {
            need_record: self.need_record,
            path: self.path.clone(),
            fragment: self.fragment,
            aof_ratio: self.aof_ratio,
            live_ts_count: self.live_ts_count,
            prefix: self.prefix.clone(),
            s3: self.s3.as_ref().map(|config| S3UploadConfig {
                bucket: config.bucket.clone(),
                prefix: config.prefix.clone(),
            }),
        }
    }
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
    pub on_hls: Option<String>,
}

impl HttpNotifierConfig {
    pub fn streamhub_config(&self) -> StreamhubHttpNotifierConfig {
        StreamhubHttpNotifierConfig {
            on_publish: self.on_publish.clone(),
            on_unpublish: self.on_unpublish.clone(),
            on_play: self.on_play.clone(),
            on_stop: self.on_stop.clone(),
            on_hls: self.on_hls.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AuthSecretConfig {
    pub key: String,
    pub password: String,
    pub push_password: Option<String>,
}
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AuthConfig {
    pub pull_enabled: bool,
    pub push_enabled: Option<bool>,
    pub algorithm: AuthAlgorithm,
}
#[derive(Debug, Deserialize, Clone)]
pub struct S3Config {
    pub enabled: bool,
    pub bucket: String,
    pub region: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub endpoint: Option<String>,
    pub prefix: Option<String>,
}

impl S3Config {
    pub fn new(
        enabled: bool,
        bucket: String,
        region: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        endpoint: Option<String>,
        prefix: Option<String>,
    ) -> Self {
        Self {
            enabled,
            bucket,
            region,
            access_key,
            secret_key,
            endpoint,
            prefix,
        }
    }
}

pub fn load(cfg_path: &String) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(cfg_path)?;
    match std::path::Path::new(cfg_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "toml" => Ok(toml::from_str(&content)?),
        "json" => Ok(serde_json::from_str(&content)?),
        extension => Err(ConfigError {
            value: ConfigErrorValue::UnsupportedFormat(extension.to_string()),
        }),
    }
}

pub fn load_from_json(json_content: &str) -> Result<Config, ConfigError> {
    Ok(serde_json::from_str(json_content)?)
}
