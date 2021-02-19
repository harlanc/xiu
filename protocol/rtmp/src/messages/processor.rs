use super::define::Rtmp_Messages;
use super::errors::MessageError;
use super::errors::MessageErrorValue;
use super::msg_types;
use crate::chunk::{self, ChunkInfo};
use liverust_lib::netio::bytes_reader::BytesReader;

use crate::amf0::amf0_markers;
use crate::amf0::amf0_reader::Amf0Reader;

use crate::protocol_control_messages::reader::ProtocolControlMessageReader;

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
        let mut reader = BytesReader::new(self.chunk_info.payload.clone());

        match self.chunk_info.message_header.msg_type_id {
            msg_types::COMMAND_AMF0 | msg_types::COMMAND_AMF3 => {
                if self.chunk_info.message_header.msg_type_id == msg_types::COMMAND_AMF0 {
                    reader.read_u8()?;
                }
                let mut amf_reader = Amf0Reader::new(reader);

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

            msg_types::SET_CHUNK_SIZE => {
                let chunk_size = ProtocolControlMessageReader::new(reader).read_set_chunk_size()?;
                return Ok(Rtmp_Messages::SET_CHUNK_SIZE {
                    chunk_size: chunk_size,
                });
            }
            msg_types::ABORT => {
                let chunk_stream_id =
                    ProtocolControlMessageReader::new(reader).read_abort_message()?;
                return Ok(Rtmp_Messages::ABORT_MESSAGE {
                    chunk_stream_id: chunk_stream_id,
                });
            }
            msg_types::ACKNOWLEDGEMENT => {
                let sequence_number =
                    ProtocolControlMessageReader::new(reader).read_acknowledgement()?;
                return Ok(Rtmp_Messages::ACKNOWLEDGEMENT {
                    sequence_number: sequence_number,
                });
            }
            msg_types::WIN_ACKNOWLEDGEMENT_SIZE => {
                let size =
                    ProtocolControlMessageReader::new(reader).read_window_acknowledgement_size()?;
                return Ok(Rtmp_Messages::WINDOW_ACKNOWLEDGEMENT_SIZE { size: size });
            }
            msg_types::SET_PEER_BANDWIDTH => {
                let properties =
                    ProtocolControlMessageReader::new(reader).read_set_peer_bandwidth()?;
                return Ok(Rtmp_Messages::SET_PEER_BANDWIDTH {
                    properties: properties,
                });
            }

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
