use super::errors::ConfigError;
use serde_derive::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rtmp: Option<RtmpConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RtmpConfig {
    pub enabled: bool,
    pub port: u32,
    pub pull: Option<RtmpPullConfig>,
    pub push: Option<RtmpPushConfig>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RtmpPullConfig {
    pub enabled: bool,
    pub address: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RtmpPushConfig {
    pub enabled: bool,
    pub address: String,
}

pub fn load() -> Result<Config, ConfigError> {
    let content = fs::read_to_string("/Users/zexu/github/xiu/application/src/config/config.toml")?;
    let decoded_config = toml::from_str(&content[..]).unwrap();
    Ok(decoded_config)
}

#[test]
fn test_toml_parse() {
    // let path = env::current_dir();
    // match path {
    //     Ok(val) => println!("The current directory is {}", val.display()),
    //     Err(err) => print!("{}", err),
    // }

    let str = fs::read_to_string("/Users/zexu/github/xiu/application/src/config/config.toml");

    match str {
        Ok(val) => {
            println!("++++++{}", val);
            let decoded: Config = toml::from_str(&val[..]).unwrap();

            let rtmp = decoded.rtmp;
            match rtmp {
                Some(val) => {
                    println!("++++++{}", val.enabled);
                }
                None => {}
            }
        }
        Err(err) => print!("======{}", err),
    }
}
