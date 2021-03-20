use netio::bytes_errors::BytesReadError;

pub enum TagParseErrorValue {
    BytesReadError(BytesReadError),
    TagDataLength
}

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
