use failure::Fail;

use flv::errors::TagParseError;

use crate::chunk::errors::PackError;

#[derive(Debug, Fail)]
pub enum CacheErrorValue {
    #[fail(display = "tag parse error")]
    TagParseError(TagParseError),
    #[fail(display = "pack error")]
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
