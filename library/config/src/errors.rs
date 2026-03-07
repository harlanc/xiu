#![allow(non_local_definitions)]
use {
    failure::{Backtrace, Fail},
    serde_json,
    std::{fmt, io::Error},
};
#[derive(Debug)]
pub struct ConfigError {
    pub value: ConfigErrorValue,
}

#[derive(Debug, Fail)]
pub enum ConfigErrorValue {
    #[fail(display = "IO error: {}", _0)]
    TomlError(toml::de::Error),
    #[fail(display = "IO error: {}", _0)]
    IOError(Error),
    #[fail(display = "JSON deserialization error: {}", _0)]
    JsonError(serde_json::Error),
    #[fail(display = "Unsupported configuration format: {}", _0)]
    UnsupportedFormat(String),
}

impl From<Error> for ConfigError {
    fn from(error: Error) -> Self {
        ConfigError {
            value: ConfigErrorValue::IOError(error),
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        ConfigError {
            value: ConfigErrorValue::JsonError(error),
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        ConfigError {
            value: ConfigErrorValue::IOError(error.into()),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for ConfigError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
