use std::io::Error;

pub struct ConfigError {
    pub value: ConfigErrorValue,
}

pub enum ConfigErrorValue {
    IOError(Error),
}

impl From<Error> for ConfigError {
    fn from(error: Error) -> Self {
        ConfigError {
            value: ConfigErrorValue::IOError(error),
        }
    }
}
