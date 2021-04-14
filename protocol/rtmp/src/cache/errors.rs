use {crate::chunk::errors::PackError, failure::Fail, flv::errors::TagParseError};

#[derive(Debug, Fail)]
pub enum CacheErrorValue {
    #[fail(display = "tag parse error\n")]
    TagParseError(TagParseError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
}

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
    #[fail(display = "tag parse error\n")]
    TagParseError(TagParseError),
    #[fail(display = "pack error\n")]
    PackError(PackError),
}

pub struct MetadataError {
    pub value: CacheErrorValue,
}
