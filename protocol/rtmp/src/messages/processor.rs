use super::errors::MessageError;
use super::msg_types;
use crate::chunk::ChunkInfo;
use liverust_lib::netio::{
    errors::IOReadError,
    reader::NetworkReader,
    reader::Reader,
    writer::{IOWriteError, Writer},
};

use crate::amf0::amf0_reader::Amf0Reader;
pub struct MessageProcessor {
    chunk_info: ChunkInfo,
}

impl MessageProcessor {
    pub fn execute(&mut self) -> Result<(), MessageError> {
        let mut reader = Reader::new(self.chunk_info.payload.clone());

        if self.chunk_info.message_header.msg_type_id == msg_types::COMMAND_AMF0 {
            reader.read_u8()?;
        }

        let mut amf_reader = Amf0Reader::new(reader);

        match self.chunk_info.message_header.msg_type_id {
            msg_types::COMMAND_AMF0 => {
                amf_reader.read_any()?;
            }
            msg_types::COMMAND_AMF3 => {}

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

            _ => {}
        }

        Ok(())
    }
}
