use super::bytes_errors::BytesReadError;
use super::bytes_errors::BytesWriteError;
use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug, Fail)]
pub enum BitErrorValue {
    #[fail(display = "bytes read error")]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
    #[fail(display = "the size is bigger than 64")]
    TooBig,
    #[fail(display = "cannot write the whole 8 bits")]
    CannotWrite8Bit,
    #[fail(display = "cannot read byte")]
    CannotReadByte,
}
#[derive(Debug)]
pub struct BitError {
    pub value: BitErrorValue,
}

impl From<BitErrorValue> for BitError {
    fn from(val: BitErrorValue) -> Self {
        BitError { value: val }
    }
}

impl From<BytesReadError> for BitError {
    fn from(error: BytesReadError) -> Self {
        BitError {
            value: BitErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for BitError {
    fn from(error: BytesWriteError) -> Self {
        BitError {
            value: BitErrorValue::BytesWriteError(error),
        }
    }
}

// impl From<Elapsed> for NetIOError {
//     fn from(error: Elapsed) -> Self {
//         NetIOError {
//             value: NetIOErrorValue::TimeoutError(error),
//         }
//     }
// }

impl fmt::Display for BitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for BitError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
