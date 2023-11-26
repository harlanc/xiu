use {
    failure::{Backtrace, Fail},
    std::{fmt, io::Error},
};
#[derive(Debug)]
pub struct ConfigError {
    pub value: ConfigErrorValue,
}

#[derive(Debug, Fail)]
pub enum ConfigErrorValue {
    #[fail(display = "IO error: {}", _0)]
    IOError(Error),
}

impl From<Error> for ConfigError {
    fn from(error: Error) -> Self {
        ConfigError {
            value: ConfigErrorValue::IOError(error),
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
