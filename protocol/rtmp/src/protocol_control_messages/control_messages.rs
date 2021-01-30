use super::errors::ControlMessagesError;
use crate::amf0::amf0_writer::Amf0Writer;
use crate::amf0::define::Amf0ValueType;

use crate::messages::rtmp_message_type;
use byteorder::{BigEndian, LittleEndian};
use liverust_lib::netio::writer::Writer;

pub struct ControlMessages {
    writer: Writer,
    amf0_writer: Amf0Writer,
}

impl ControlMessages {
    fn write_control_message_header(
        &mut self,
        msg_type_id: u8,
        len: u32,
    ) -> Result<(), ControlMessagesError> {
        //0 1 2 3 4 5 6 7
        //+-+-+-+-+-+-+-+-+
        //|fmt|  cs id  |
        //+-+-+-+-+-+-+-+-+
        // 0x0     0x02
        self.writer.write_u8(0x0 << 6 | 0x02)?; //fmt 0 and csid 2
        self.writer.write_u24::<BigEndian>(0)?; //timestamp 3 bytes and value 0
        self.writer.write_u32::<BigEndian>(len)?; //msg length
        self.writer.write_u8(msg_type_id)?; //msg type id
        self.writer.write_u32::<BigEndian>(0)?; //msg stream ID 0
        Ok(())
    }
    fn set_chunk_size(&mut self, chunk_size: u32) -> Result<(), ControlMessagesError> {
        self.write_control_message_header(rtmp_message_type::RTMP_MSG_TYPE_SET_CHUNK_SIZE, 4)?;
        self.writer
            .write_u32::<BigEndian>(chunk_size & 0x7FFFFFFF)?; //first bit must be 0
        Ok(())
    }

    fn abort_message(&mut self, chunk_stream_id: u32) -> Result<(), ControlMessagesError> {
        self.write_control_message_header(rtmp_message_type::RTMP_MSG_TYPE_ABORT, 4)?;
        self.writer.write_u32::<BigEndian>(chunk_stream_id)?;

        Ok(())
    }

    fn acknowledgement(&mut self, sequence_number: u32) -> Result<(), ControlMessagesError> {
        self.write_control_message_header(rtmp_message_type::RTMP_MSG_TYPE_ACKNOWLEDGEMENT, 4)?;
        self.writer.write_u32::<BigEndian>(sequence_number)?;

        Ok(())
    }

    fn window_acknowledgement_size(
        &mut self,
        window_size: u32,
    ) -> Result<(), ControlMessagesError> {
        self.write_control_message_header(rtmp_message_type::RTMP_MSG_TYPE_WIN_ACKNOWLEDGEMENT_SIZE, 4)?;
        self.writer.write_u32::<BigEndian>(window_size)?;

        Ok(())
    }

    fn set_peer_bandwidth(
        &mut self,
        window_size: u32,
        limit_type: u8,
    ) -> Result<(), ControlMessagesError> {
        self.write_control_message_header(rtmp_message_type::RTMP_MSG_TYPE_SET_PEER_BANDWIDTH, 4)?;
        self.writer.write_u32::<BigEndian>(window_size)?;
        self.writer.write_u8(limit_type)?;

        Ok(())
    }
}
