use netio::bytes_errors::BytesReadError;
use netio::bytes_errors::BytesWriteError;

pub struct ControlMessagesError {
    pub value: ControlMessagesErrorValue,
}

pub enum ControlMessagesErrorValue {
    //Amf0WriteError(Amf0WriteError),
    BytesWriteError(BytesWriteError),
}

// impl From<Amf0WriteError> for ControlMessagesError {
//     fn from(error: Amf0WriteError) -> Self {
//         ControlMessagesError {
//             value: ControlMessagesErrorValue::Amf0WriteError(error),
//         }
//     }
// }

impl From<BytesWriteError> for ControlMessagesError {
    fn from(error: BytesWriteError) -> Self {
        ControlMessagesError {
            value: ControlMessagesErrorValue::BytesWriteError(error),
        }
    }
}

pub struct ProtocolControlMessageReaderError {
    pub value: ProtocolControlMessageReaderErrorValue,
}

pub enum ProtocolControlMessageReaderErrorValue {
    BytesReadError(BytesReadError),
}

impl From<BytesReadError> for ProtocolControlMessageReaderError {
    fn from(error: BytesReadError) -> Self {
        ProtocolControlMessageReaderError {
            value: ProtocolControlMessageReaderErrorValue::BytesReadError(error),
        }
    }
}
