use super::define::msg_type_id;
use super::define::RtmpMessageData;
use super::errors::MessageError;
use super::errors::MessageErrorValue;
use crate::chunk::ChunkInfo;
use netio::bytes_reader::BytesReader;

use crate::amf0::amf0_markers;
use crate::amf0::amf0_reader::Amf0Reader;

use crate::protocol_control_messages::reader::ProtocolControlMessageReader;

pub struct MessageParser {
    chunk_info: ChunkInfo,
}

impl MessageParser {
    pub fn new(chunk_info: ChunkInfo) -> Self {
        Self {
            chunk_info: chunk_info,
        }
    }
    pub fn parse(&mut self) -> Result<RtmpMessageData, MessageError> {
        let mut reader = BytesReader::new(self.chunk_info.payload.clone());

        match self.chunk_info.message_header.msg_type_id {
            msg_type_id::COMMAND_AMF0 | msg_type_id::COMMAND_AMF3 => {
                if self.chunk_info.message_header.msg_type_id == msg_type_id::COMMAND_AMF3 {
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

                return Ok(RtmpMessageData::Amf0Command {
                    command_name: command_name,
                    transaction_id: transaction_id,
                    command_object: command_obj,
                    others,
                });
            }
            // msg_types::COMMAND_AMF3 => {}
            msg_type_id::AUDIO => {
                return Ok(RtmpMessageData::AudioData {
                    data: self.chunk_info.payload.clone(),
                })
            }
            msg_type_id::VIDEO => {
                return Ok(RtmpMessageData::VideoData {
                    data: self.chunk_info.payload.clone(),
                })
            }

            msg_type_id::USER_CONTROL_EVENT => {}

            msg_type_id::SET_CHUNK_SIZE => {
                let chunk_size = ProtocolControlMessageReader::new(reader).read_set_chunk_size()?;
                return Ok(RtmpMessageData::SetChunkSize {
                    chunk_size: chunk_size,
                });
            }
            msg_type_id::ABORT => {
                let chunk_stream_id =
                    ProtocolControlMessageReader::new(reader).read_abort_message()?;
                return Ok(RtmpMessageData::AbortMessage {
                    chunk_stream_id: chunk_stream_id,
                });
            }
            msg_type_id::ACKNOWLEDGEMENT => {
                let sequence_number =
                    ProtocolControlMessageReader::new(reader).read_acknowledgement()?;
                return Ok(RtmpMessageData::Acknowledgement {
                    sequence_number: sequence_number,
                });
            }
            msg_type_id::WIN_ACKNOWLEDGEMENT_SIZE => {
                let size =
                    ProtocolControlMessageReader::new(reader).read_window_acknowledgement_size()?;
                return Ok(RtmpMessageData::WindowAcknowledgementSize { size: size });
            }
            msg_type_id::SET_PEER_BANDWIDTH => {
                let properties =
                    ProtocolControlMessageReader::new(reader).read_set_peer_bandwidth()?;
                return Ok(RtmpMessageData::SetPeerBandwidth {
                    properties: properties,
                });
            }

            msg_type_id::DATA_AMF0 | msg_type_id::DATA_AMF3 => {}

            msg_type_id::SHARED_OBJ_AMF3 | msg_type_id::SHARED_OBJ_AMF0 => {}

            msg_type_id::AGGREGATE => {}

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
