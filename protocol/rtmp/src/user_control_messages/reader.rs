use super::define;
use super::errors;
use crate::messages::define as message_define;
use byteorder::BigEndian;
use netio::bytes_reader::BytesReader;

pub struct EventMessagesReader {
    pub reader: BytesReader,
}

impl EventMessagesReader {
    pub fn new(reader: BytesReader) -> Self {
        Self { reader: reader }
    }

    pub fn parse_event(
        &mut self,
    ) -> Result<message_define::RtmpMessageData, errors::EventMessagesError> {
        let event_type = self.reader.read_u16::<BigEndian>()?;
        match event_type {
            define::RTMP_EVENT_SET_BUFFER_LENGTH => {
                return self.read_set_buffer_length();
            }

            _ => {
                return Err(errors::EventMessagesError {
                    value: errors::EventMessagesErrorValue::UnknowEventMessageType,
                })
            }
        }
    }
    pub fn read_set_buffer_length(
        &mut self,
    ) -> Result<message_define::RtmpMessageData, errors::EventMessagesError> {
        let stream_id = self.reader.read_u32::<BigEndian>()?;
        let ms = self.reader.read_u32::<BigEndian>()?;

        return Ok(message_define::RtmpMessageData::SetBufferLength {
            stream_id: stream_id,
            buffer_length: ms,
        });
    }
}
