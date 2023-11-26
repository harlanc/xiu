use {
    failure::{Backtrace, Fail},
    bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    std::fmt,
};

#[derive(Debug)]
pub struct ControlMessagesError {
    pub value: ControlMessagesErrorValue,
}

#[derive(Debug, Fail)]
pub enum ControlMessagesErrorValue {
    //Amf0WriteError(Amf0WriteError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(BytesWriteError),
}

impl From<BytesWriteError> for ControlMessagesError {
    fn from(error: BytesWriteError) -> Self {
        ControlMessagesError {
            value: ControlMessagesErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for ControlMessagesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for ControlMessagesError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[derive(Debug)]
pub struct ProtocolControlMessageReaderError {
    pub value: ProtocolControlMessageReaderErrorValue,
}

#[derive(Debug, Fail)]
pub enum ProtocolControlMessageReaderErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
}

impl From<BytesReadError> for ProtocolControlMessageReaderError {
    fn from(error: BytesReadError) -> Self {
        ProtocolControlMessageReaderError {
            value: ProtocolControlMessageReaderErrorValue::BytesReadError(error),
        }
    }
}

impl fmt::Display for ProtocolControlMessageReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for ProtocolControlMessageReaderError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
