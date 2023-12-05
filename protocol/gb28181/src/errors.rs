use {
    failure::{Backtrace, Fail},
    std::fmt,
};

#[derive(Debug)]
pub struct Gb28181Error {
    pub value: Gb28181ErrorValue,
}

#[derive(Debug, Fail)]
pub enum Gb28181ErrorValue {
    #[fail(display = "The session name alreay exists.")]
    SessionExists,
    #[fail(display = "New server session failed.")]
    NewSessionFailed,
}

impl fmt::Display for Gb28181Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for Gb28181Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
