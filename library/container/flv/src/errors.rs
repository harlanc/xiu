use {
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::{Backtrace, Fail},
    std::fmt,
};

#[derive(Debug, Fail)]
pub enum TagParseErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),
    #[fail(display = "tag data length error\n")]
    TagDataLength,
    #[fail(display = "unknow tag type error\n")]
    UnknownTagType,
}
#[derive(Debug)]
pub struct TagParseError {
    pub value: TagParseErrorValue,
}

impl From<BytesReadError> for TagParseError {
    fn from(error: BytesReadError) -> Self {
        TagParseError {
            value: TagParseErrorValue::BytesReadError(error),
        }
    }
}

impl fmt::Display for TagParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for TagParseError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
#[derive(Debug)]
pub struct MuxerError {
    pub value: MuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum MuxerErrorValue {
    // #[fail(display = "server error")]
    // Error,
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
}

impl From<BytesWriteError> for MuxerError {
    fn from(error: BytesWriteError) -> Self {
        MuxerError {
            value: MuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for MuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug)]
pub struct FlvDemuxerError {
    pub value: DemuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum DemuxerErrorValue {
    // #[fail(display = "server error")]
    // Error,
    #[fail(display = "bytes write error:{}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "bytes read error:{}\n", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "mpeg avc error:{}\n", _0)]
    MpegAvcError(#[cause] MpegAvcError),
    #[fail(display = "mpeg aac error:{}\n", _0)]
    MpegAacError(#[cause] MpegAacError),
}

impl From<BytesWriteError> for FlvDemuxerError {
    fn from(error: BytesWriteError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BytesReadError> for FlvDemuxerError {
    fn from(error: BytesReadError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::BytesReadError(error),
        }
    }
}

impl From<MpegAvcError> for FlvDemuxerError {
    fn from(error: MpegAvcError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::MpegAvcError(error),
        }
    }
}

impl From<MpegAacError> for FlvDemuxerError {
    fn from(error: MpegAacError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::MpegAacError(error),
        }
    }
}

impl fmt::Display for FlvDemuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for FlvDemuxerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[derive(Debug, Fail)]
pub enum MpegErrorValue {
    #[fail(display = "bytes read error:{}\n", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error:{}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "there is not enough bits to read\n")]
    NotEnoughBitsToRead,
    #[fail(display = "should not come here\n")]
    ShouldNotComeHere,
}
#[derive(Debug)]
pub struct MpegAvcError {
    pub value: MpegErrorValue,
}

impl From<BytesReadError> for MpegAvcError {
    fn from(error: BytesReadError) -> Self {
        MpegAvcError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegAvcError {
    fn from(error: BytesWriteError) -> Self {
        MpegAvcError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for MpegAvcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MpegAvcError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[derive(Debug)]
pub struct MpegAacError {
    pub value: MpegErrorValue,
}

impl From<BytesReadError> for MpegAacError {
    fn from(error: BytesReadError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegAacError {
    fn from(error: BytesWriteError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for MpegAacError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for MpegAacError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[derive(Debug, Fail)]
pub enum BitVecErrorValue {
    #[fail(display = "not enough bits left\n")]
    NotEnoughBits,
}
#[derive(Debug)]
pub struct BitVecError {
    pub value: BitVecErrorValue,
}
