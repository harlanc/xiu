use {
    crate::{amf0::errors::Amf0WriteError, chunk::errors::PackError},
    failure::Fail,
    xflv::errors::FlvDemuxerError,
    std::fmt,
};

#[derive(Debug, Fail)]
pub enum CacheErrorValue {
    #[fail(display = "cache tag parse error\n")]
    DemuxerError(FlvDemuxerError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
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
