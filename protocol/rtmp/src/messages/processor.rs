use super::errors::MessageError;
use super::errors::MessageErrorValue;
use super::messages::Rtmp_Messages;
use super::msg_types;
use crate::{amf0, chunk::ChunkInfo};
use liverust_lib::netio::{
    errors::IOReadError,
    reader::NetworkReader,
    reader::Reader,
    writer::{IOWriteError, Writer},
};

use crate::amf0::amf0_markers;
use crate::amf0::amf0_reader::Amf0Reader;
use crate::amf0::Amf0ReadError;
use crate::amf0::Amf0ValueType;
pub struct MessageProcessor {
    chunk_info: ChunkInfo,
}

impl MessageProcessor {
    pub fn new(chunk_info: ChunkInfo) -> Self {
        Self {
            chunk_info: chunk_info,
        }
    }
    pub fn execute(&mut self) -> Result<Rtmp_Messages, MessageError> {
        let mut reader = Reader::new(self.chunk_info.payload.clone());

        if self.chunk_info.message_header.msg_type_id == msg_types::COMMAND_AMF0 {
            reader.read_u8()?;
        }

        let mut amf_reader = Amf0Reader::new(reader);

        match self.chunk_info.message_header.msg_type_id {
            msg_types::COMMAND_AMF0 | msg_types::COMMAND_AMF3 => {
                let command_name = amf_reader.read_with_type(amf0_markers::STRING)?;
                let transaction_id = amf_reader.read_with_type(amf0_markers::NUMBER)?;

                //The third value can be an object or NULL object
                let command_obj_raw = amf_reader.read_with_type(amf0_markers::OBJECT);
                let command_obj = match command_obj_raw {
                    Ok(val) => val,
                    Err(_) => amf_reader.read_with_type(amf0_markers::NULL)?,
                };

                let others = amf_reader.read_all()?;

                return Ok(Rtmp_Messages::AMF0_COMMAND {
                    msg_stream_id: self.chunk_info.message_header.msg_streamd_id,
                    command_name: command_name,
                    transaction_id: transaction_id,
                    command_object: command_obj,
                    others,
                });
            }
            // msg_types::COMMAND_AMF3 => {}
            msg_types::AUDIO => {}
            msg_types::VIDEO => {}

            msg_types::USER_CONTROL_EVENT => {}

            msg_types::SET_CHUNK_SIZE
            | msg_types::ABORT
            | msg_types::ACKNOWLEDGEMENT
            | msg_types::WIN_ACKNOWLEDGEMENT_SIZE
            | msg_types::SET_PEER_BANDWIDTH => {}

            msg_types::DATA_AMF0 | msg_types::DATA_AMF3 => {}

            msg_types::SHARED_OBJ_AMF3 | msg_types::SHARED_OBJ_AMF0 => {}

            msg_types::AGGREGATE => {}

            _ => {
                return Err(MessageError {
                    value: MessageErrorValue::UnknowMessageType,
                });
            }
        }
        return Err(MessageError {
            value: MessageErrorValue::UnknowMessageType,
        });
    }
}
