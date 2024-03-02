use {
    audiopus::error::Error as OpusError,
    failure::{Backtrace, Fail},
    fdk_aac::enc::EncoderError as AacEncoderError,
    std::fmt,
    std::num::ParseIntError,
    webrtc::error::Error as RTCError,
    webrtc::util::Error as RTCUtilError,
};

#[derive(Debug)]
pub struct WebRTCError {
    pub value: WebRTCErrorValue,
}

#[derive(Debug, Fail)]
pub enum WebRTCErrorValue {
    #[fail(display = "webrtc error: {}", _0)]
    RTCError(#[cause] RTCError),
    #[fail(display = "webrtc util error: {}", _0)]
    RTCUtilError(#[cause] RTCUtilError),
    #[fail(display = "webrtc util error: {}", _0)]
    ParseIntError(#[cause] ParseIntError),
    #[fail(display = "cannot get local description")]
    CanNotGetLocalDescription,
    #[fail(display = "opus2aac error")]
    Opus2AacError,
    #[fail(display = "missing whitespace")]
    MissingWhitespace,
    #[fail(display = "missing colon")]
    MissingColon,
}

impl From<RTCError> for WebRTCError {
    fn from(error: RTCError) -> Self {
        WebRTCError {
            value: WebRTCErrorValue::RTCError(error),
        }
    }
}

impl From<RTCUtilError> for WebRTCError {
    fn from(error: RTCUtilError) -> Self {
        WebRTCError {
            value: WebRTCErrorValue::RTCUtilError(error),
        }
    }
}

impl From<ParseIntError> for WebRTCError {
    fn from(error: ParseIntError) -> Self {
        WebRTCError {
            value: WebRTCErrorValue::ParseIntError(error),
        }
    }
}

impl fmt::Display for WebRTCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for WebRTCError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[derive(Debug)]
pub struct Opus2AacError {
    pub value: Opus2AacErrorValue,
}

#[derive(Debug)]
pub enum Opus2AacErrorValue {
    OpusError(OpusError),
    AacEncoderError(AacEncoderError),
}

impl From<OpusError> for Opus2AacError {
    fn from(error: OpusError) -> Self {
        Opus2AacError {
            value: Opus2AacErrorValue::OpusError(error),
        }
    }
}

impl From<AacEncoderError> for Opus2AacError {
    fn from(error: AacEncoderError) -> Self {
        Opus2AacError {
            value: Opus2AacErrorValue::AacEncoderError(error),
        }
    }
}

// impl fmt::Display for Opus2AacError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(&self.value, f)
//     }
// }
