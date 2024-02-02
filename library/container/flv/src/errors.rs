use {
    bytesio::bits_errors::BitError,
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    failure::{Backtrace, Fail},
    h264_decoder::errors::H264Error,
    std::fmt,
};

#[derive(Debug, Fail)]
pub enum TagParseErrorValue {
    #[fail(display = "bytes read error")]
    BytesReadError(BytesReadError),
    #[fail(display = "tag data length error")]
    TagDataLength,
    #[fail(display = "unknow tag type error")]
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
pub struct FlvMuxerError {
    pub value: MuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum MuxerErrorValue {
    // #[fail(display = "server error")]
    // Error,
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
}

impl From<BytesWriteError> for FlvMuxerError {
    fn from(error: BytesWriteError) -> Self {
        FlvMuxerError {
            value: MuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for FlvMuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for FlvMuxerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
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
    #[fail(display = "bytes write error:{}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "bytes read error:{}", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "mpeg avc error:{}", _0)]
    MpegAvcError(#[cause] Mpeg4AvcHevcError),
    #[fail(display = "mpeg aac error:{}", _0)]
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

impl From<Mpeg4AvcHevcError> for FlvDemuxerError {
    fn from(error: Mpeg4AvcHevcError) -> Self {
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
    #[fail(display = "bytes read error:{}", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error:{}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "bits error:{}", _0)]
    BitError(#[cause] BitError),
    #[fail(display = "h264 error:{}", _0)]
    H264Error(#[cause] H264Error),
    #[fail(display = "there is not enough bits to read")]
    NotEnoughBitsToRead,
    #[fail(display = "should not come here")]
    ShouldNotComeHere,
    #[fail(display = "the sps nal unit type is not correct")]
    SPSNalunitTypeNotCorrect,
    #[fail(display = "not supported sampling frequency")]
    NotSupportedSamplingFrequency,
}
#[derive(Debug)]
pub struct Mpeg4AvcHevcError {
    pub value: MpegErrorValue,
}

impl From<BytesReadError> for Mpeg4AvcHevcError {
    fn from(error: BytesReadError) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for Mpeg4AvcHevcError {
    fn from(error: BytesWriteError) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl From<H264Error> for Mpeg4AvcHevcError {
    fn from(error: H264Error) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::H264Error(error),
        }
    }
}

impl fmt::Display for Mpeg4AvcHevcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for Mpeg4AvcHevcError {
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

impl From<BitError> for MpegAacError {
    fn from(error: BitError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BitError(error),
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
    #[fail(display = "not enough bits left")]
    NotEnoughBits,
}
#[derive(Debug)]
pub struct BitVecError {
    pub value: BitVecErrorValue,
}
