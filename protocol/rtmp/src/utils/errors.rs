use {
    failure::{Backtrace, Fail},
    std::fmt,
};

#[derive(Debug)]
pub struct RtmpUrlParseError {
    pub value: RtmpUrlParseErrorValue,
}
#[derive(Debug, Fail)]
pub enum RtmpUrlParseErrorValue {
    #[fail(display = "The url is not valid")]
    Notvalid,
}

impl fmt::Display for RtmpUrlParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for RtmpUrlParseError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
