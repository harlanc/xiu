use crate::amf0::errors::{self, Amf0WriteError};
use crate::chunk::errors::UnpackError;
use crate::messages::errors::MessageError;
use crate::netconnection::errors::NetConnectionError;
use crate::netstream::errors::NetStreamError;
use crate::protocol_control_messages::errors::ControlMessagesError;
use crate::user_control_messages::errors::EventMessagesError;
use crate::chunk::errors::PackError;
use crate::handshake::errors::HandshakeError;

use liverust_lib::netio::bytes_errors::BytesWriteError;
use liverust_lib::netio::netio_errors::NetIOError;

use tokio::time::Elapsed;



pub struct ServerError {
    pub value: ServerErrorValue,
}

pub enum ServerErrorValue {
    Amf0WriteError(Amf0WriteError),
    BytesWriteError(BytesWriteError),
    TimeoutError(Elapsed),
    UnPackError(UnpackError),
    MessageError(MessageError),
    ControlMessagesError(ControlMessagesError),
    NetConnectionError(NetConnectionError),
    NetStreamError(NetStreamError),
    EventMessagesError(EventMessagesError),
    NetIOError(NetIOError),
    PackError(PackError),
    Amf0ValueCountNotCorrect,
    Amf0ValueTypeNotCorrect,
}

impl From<Amf0WriteError> for ServerError {
    fn from(error: Amf0WriteError) -> Self {
        ServerError {
            value: ServerErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<BytesWriteError> for ServerError {
    fn from(error: BytesWriteError) -> Self {
        ServerError {
            value: ServerErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Elapsed> for ServerError {
    fn from(error: Elapsed) -> Self {
        ServerError {
            value: ServerErrorValue::TimeoutError(error),
        }
    }
}

impl From<UnpackError> for ServerError {
    fn from(error: UnpackError) -> Self {
        ServerError {
            value: ServerErrorValue::UnPackError(error),
        }
    }
}

impl From<MessageError> for ServerError {
    fn from(error: MessageError) -> Self {
        ServerError {
            value: ServerErrorValue::MessageError(error),
        }
    }
}

impl From<ControlMessagesError> for ServerError {
    fn from(error: ControlMessagesError) -> Self {
        ServerError {
            value: ServerErrorValue::ControlMessagesError(error),
        }
    }
}

impl From<NetConnectionError> for ServerError {
    fn from(error: NetConnectionError) -> Self {
        ServerError {
            value: ServerErrorValue::NetConnectionError(error),
        }
    }
}

impl From<NetStreamError> for ServerError {
    fn from(error: NetStreamError) -> Self {
        ServerError {
            value: ServerErrorValue::NetStreamError(error),
        }
    }
}

impl From<EventMessagesError> for ServerError {
    fn from(error: EventMessagesError) -> Self {
        ServerError {
            value: ServerErrorValue::EventMessagesError(error),
        }
    }
}

impl From<NetIOError> for ServerError {
    fn from(error: NetIOError) -> Self {
        ServerError {
            value: ServerErrorValue::NetIOError(error),
        }
    }
}

pub struct ClientError {
    pub value: ClientErrorValue,
}

pub enum ClientErrorValue {
    Amf0WriteError(Amf0WriteError),
    BytesWriteError(BytesWriteError),
    TimeoutError(Elapsed),
    UnPackError(UnpackError),
    MessageError(MessageError),
    ControlMessagesError(ControlMessagesError),
    NetConnectionError(NetConnectionError),
    NetStreamError(NetStreamError),
    EventMessagesError(EventMessagesError),
    NetIOError(NetIOError),
    PackError(PackError),
    HandshakeError(HandshakeError),

    Amf0ValueCountNotCorrect,
    Amf0ValueTypeNotCorrect,
}

impl From<Amf0WriteError> for ClientError {
    fn from(error: Amf0WriteError) -> Self {
        ClientError {
            value: ClientErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<BytesWriteError> for ClientError {
    fn from(error: BytesWriteError) -> Self {
        ClientError {
            value: ClientErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Elapsed> for ClientError {
    fn from(error: Elapsed) -> Self {
        ClientError {
            value: ClientErrorValue::TimeoutError(error),
        }
    }
}

impl From<UnpackError> for ClientError {
    fn from(error: UnpackError) -> Self {
        ClientError {
            value: ClientErrorValue::UnPackError(error),
        }
    }
}

impl From<MessageError> for ClientError {
    fn from(error: MessageError) -> Self {
        ClientError {
            value: ClientErrorValue::MessageError(error),
        }
    }
}

impl From<ControlMessagesError> for ClientError {
    fn from(error: ControlMessagesError) -> Self {
        ClientError {
            value: ClientErrorValue::ControlMessagesError(error),
        }
    }
}

impl From<NetConnectionError> for ClientError {
    fn from(error: NetConnectionError) -> Self {
        ClientError {
            value: ClientErrorValue::NetConnectionError(error),
        }
    }
}

impl From<NetStreamError> for ClientError {
    fn from(error: NetStreamError) -> Self {
        ClientError {
            value: ClientErrorValue::NetStreamError(error),
        }
    }
}

impl From<EventMessagesError> for ClientError {
    fn from(error: EventMessagesError) -> Self {
        ClientError {
            value: ClientErrorValue::EventMessagesError(error),
        }
    }
}

impl From<NetIOError> for ClientError {
    fn from(error: NetIOError) -> Self {
        ClientError {
            value: ClientErrorValue::NetIOError(error),
        }
    }
}

impl From<PackError> for ClientError {
    fn from(error: PackError) -> Self {
        ClientError {
            value: ClientErrorValue::PackError(error),
        }
    }
}

impl From<HandshakeError> for ClientError {
    fn from(error: HandshakeError) -> Self {
        ClientError {
            value: ClientErrorValue::HandshakeError(error),
        }
    }
}


