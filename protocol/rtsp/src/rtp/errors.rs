use {
    failure::{Backtrace, Fail},
    std::fmt,
};

use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;

// #[derive(Debug)]
// pub struct RtpH264PackerError {
//     pub value: RtpH264PackerErrorValue,
// }
// #[derive(Debug, Fail)]
// pub enum RtpH264PackerErrorValue {
//     #[fail(display = "bytes read error: {}\n", _0)]
//     BytesReadError(BytesReadError),
//     #[fail(display = "bytes write error: {}\n", _0)]
//     BytesWriteError(BytesWriteError),
// }

// impl From<BytesReadError> for RtpH264PackerError {
//     fn from(error: BytesReadError) -> Self {
//         RtpH264PackerError {
//             value: RtpH264PackerErrorValue::BytesReadError(error),
//         }
//     }
// }

// impl From<BytesWriteError> for RtpH264PackerError {
//     fn from(error: BytesWriteError) -> Self {
//         RtpH264PackerError {
//             value: RtpH264PackerErrorValue::BytesWriteError(error),
//         }
//     }
// }

// impl fmt::Display for RtpH264PackerError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(&self.value, f)
//     }
// }

// impl Fail for RtpH264PackerError {
//     fn cause(&self) -> Option<&dyn Fail> {
//         self.value.cause()
//     }

//     fn backtrace(&self) -> Option<&Backtrace> {
//         self.value.backtrace()
//     }
// }

// #[derive(Debug)]
// pub struct RtpH265PackerError {
//     pub value: RtpH265PackerErrorValue,
// }
// #[derive(Debug, Fail)]
// pub enum RtpH265PackerErrorValue {
//     #[fail(display = "bytes read error: {}\n", _0)]
//     BytesReadError(BytesReadError),
//     #[fail(display = "bytes write error: {}\n", _0)]
//     BytesWriteError(BytesWriteError),
// }

// impl From<BytesReadError> for RtpH265PackerError {
//     fn from(error: BytesReadError) -> Self {
//         RtpH265PackerError {
//             value: RtpH265PackerErrorValue::BytesReadError(error),
//         }
//     }
// }

// impl From<BytesWriteError> for RtpH265PackerError {
//     fn from(error: BytesWriteError) -> Self {
//         RtpH265PackerError {
//             value: RtpH265PackerErrorValue::BytesWriteError(error),
//         }
//     }
// }

// impl fmt::Display for RtpH265PackerError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(&self.value, f)
//     }
// }

// impl Fail for RtpH265PackerError {
//     fn cause(&self) -> Option<&dyn Fail> {
//         self.value.cause()
//     }

//     fn backtrace(&self) -> Option<&Backtrace> {
//         self.value.backtrace()
//     }
// }

// #[derive(Debug)]
// pub struct RtpPackerError {
//     pub value: RtpPackerErrorValue,
// }

// #[derive(Debug, Fail)]
// pub enum RtpPackerErrorValue {
//     #[fail(display = "h264 pack error: {}\n", _0)]
//     RtpH264PackerError(RtpH264PackerError),
//     #[fail(display = "h265 pack error: {}\n", _0)]
//     RtpH265PackerError(RtpH265PackerError),
// }

// impl From<RtpH264PackerError> for RtpPackerError {
//     fn from(error: RtpH264PackerError) -> Self {
//         RtpPackerError {
//             value: RtpPackerErrorValue::RtpH264PackerError(error),
//         }
//     }
// }

// impl From<RtpH265PackerError> for RtpPackerError {
//     fn from(error: RtpH265PackerError) -> Self {
//         RtpPackerError {
//             value: RtpPackerErrorValue::RtpH265PackerError(error),
//         }
//     }
// }

// impl fmt::Display for RtpPackerError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(&self.value, f)
//     }
// }

#[derive(Debug)]
pub struct PackerError {
    pub value: PackerErrorValue,
}

impl Fail for PackerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for PackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PackerErrorValue {
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for PackerError {
    fn from(error: BytesReadError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for PackerError {
    fn from(error: BytesWriteError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesWriteError(error),
        }
    }
}

#[derive(Debug)]
pub struct UnPackerError {
    pub value: UnPackerErrorValue,
}

#[derive(Debug, Fail)]
pub enum UnPackerErrorValue {
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for UnPackerError {
    fn from(error: BytesReadError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for UnPackerError {
    fn from(error: BytesWriteError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesWriteError(error),
        }
    }
}
