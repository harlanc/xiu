use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug)]
pub struct AuthError {
    pub value: AuthErrorValue,
}

#[derive(Debug, Fail)]
pub enum AuthErrorValue {
    #[fail(display = "token is not correct.")]
    TokenIsNotCorrect,
    #[fail(display = "no token found.")]
    NoTokenFound,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for AuthError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
