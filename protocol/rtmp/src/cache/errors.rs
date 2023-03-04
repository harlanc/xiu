use {
    crate::{amf0::errors::Amf0WriteError, chunk::errors::PackError},
    bytesio::bytes_errors::BytesReadError,
    failure::{Backtrace, Fail},
    h264::errors::H264Error,
    std::fmt,
    xflv::errors::{FlvDemuxerError, MpegAacError, MpegAvcError},
};

#[derive(Debug, Fail)]
pub enum CacheErrorValue {
    #[fail(display = "cache tag parse error\n")]
    DemuxerError(FlvDemuxerError),
    #[fail(display = "mpeg aac error\n")]
    MpegAacError(MpegAacError),
    #[fail(display = "mpeg avc error\n")]
    MpegAvcError(MpegAvcError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
    #[fail(display = "read bytes error\n")]
    BytesReadError(BytesReadError),
    #[fail(display = "h264 error\n")]
    H264Error(H264Error),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
#[derive(Debug)]
pub struct CacheError {
    pub value: CacheErrorValue,
}

impl From<FlvDemuxerError> for CacheError {
    fn from(error: FlvDemuxerError) -> Self {
        CacheError {
            value: CacheErrorValue::DemuxerError(error),
        }
    }
}

impl From<H264Error> for CacheError {
    fn from(error: H264Error) -> Self {
        CacheError {
            value: CacheErrorValue::H264Error(error),
        }
    }
}

impl From<MpegAacError> for CacheError {
    fn from(error: MpegAacError) -> Self {
        CacheError {
            value: CacheErrorValue::MpegAacError(error),
        }
    }
}

impl From<MpegAvcError> for CacheError {
    fn from(error: MpegAvcError) -> Self {
        CacheError {
            value: CacheErrorValue::MpegAvcError(error),
        }
    }
}

impl From<BytesReadError> for CacheError {
    fn from(error: BytesReadError) -> Self {
        CacheError {
            value: CacheErrorValue::BytesReadError(error),
        }
    }
}

impl From<PackError> for CacheError {
    fn from(error: PackError) -> Self {
        CacheError {
            value: CacheErrorValue::PackError(error),
        }
    }
}

#[derive(Debug, Fail)]
pub enum MetadataErrorValue {
    #[fail(display = "metadata tag parse error\n")]
    DemuxerError(FlvDemuxerError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
    #[fail(display = "amf write error\n")]
    Amf0WriteError(Amf0WriteError),
}
#[derive(Debug)]
pub struct MetadataError {
    pub value: MetadataErrorValue,
}

impl From<Amf0WriteError> for MetadataError {
    fn from(error: Amf0WriteError) -> Self {
        MetadataError {
            value: MetadataErrorValue::Amf0WriteError(error),
        }
    }
}

impl Fail for MetadataError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
