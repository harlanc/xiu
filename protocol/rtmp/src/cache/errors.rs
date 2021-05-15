use {crate::chunk::errors::PackError, failure::Fail, flvparser::errors::TagParseError, std::fmt};

#[derive(Debug, Fail)]
pub enum CacheErrorValue {
    #[fail(display = "cache tag parse error\n")]
    TagParseError(TagParseError),
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

impl From<TagParseError> for CacheError {
    fn from(error: TagParseError) -> Self {
        CacheError {
            value: CacheErrorValue::TagParseError(error),
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
    TagParseError(TagParseError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
}
#[derive(Debug)]
pub struct MetadataError {
    pub value: CacheErrorValue,
}
