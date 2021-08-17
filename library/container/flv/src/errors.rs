use failure::{Backtrace, Fail};
use std::fmt;

use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;

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
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),
    #[fail(display = "mpeg avc error\n")]
    MpegAvcError(MpegAvcError),
    #[fail(display = "mpeg aac error\n")]
    MpegAacError(MpegAacError),
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

#[derive(Debug, Fail)]
pub enum MpegAacErrorValue {
    #[fail(display = "bytes read error\n")]
    BytesReadError(BytesReadError),

    #[fail(display = "bytes write error\n")]
    BytesWriteError(BytesWriteError),

    #[fail(display = "there is not enough bits to read\n")]
    NotEnoughBitsToRead,

    #[fail(display = "should not come here\n")]
    ShouldNotComeHere,
}
#[derive(Debug)]
pub struct MpegAvcError {
    pub value: MpegAacErrorValue,
}

impl From<BytesReadError> for MpegAvcError {
    fn from(error: BytesReadError) -> Self {
        MpegAvcError {
            value: MpegAacErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegAvcError {
    fn from(error: BytesWriteError) -> Self {
        MpegAvcError {
            value: MpegAacErrorValue::BytesWriteError(error),
        }
    }
}

#[derive(Debug)]
pub struct MpegAacError {
    pub value: MpegAacErrorValue,
}

impl From<BytesReadError> for MpegAacError {
    fn from(error: BytesReadError) -> Self {
        MpegAacError {
            value: MpegAacErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegAacError {
    fn from(error: BytesWriteError) -> Self {
        MpegAacError {
            value: MpegAacErrorValue::BytesWriteError(error),
        }
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
