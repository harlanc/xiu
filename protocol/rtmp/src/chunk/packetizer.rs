use byteorder::{BigEndian, LittleEndian};

use super::chunk::{ChunkBasicHeader, ChunkHeader, ChunkInfo, ChunkMessageHeader};
use liverust_lib::netio::bytes_errors::BytesWriteError;
use liverust_lib::netio::bytes_writer::AsyncBytesWriter;
use std::collections::HashMap;

use tokio::prelude::*;

#[derive(Eq, PartialEq, Debug)]
pub enum PackResult {
    Success,
    NotEnoughBytes,
}

pub enum PackErrorValue {
    NotExistHeader,
    UnknowReadState,
    // IO(io::Error),
    BytesWriteError(BytesWriteError),
}

pub struct PackError {
    pub value: PackErrorValue,
}

impl From<PackErrorValue> for PackError {
    fn from(val: PackErrorValue) -> Self {
        PackError { value: val }
    }
}

impl From<BytesWriteError> for PackError {
    fn from(error: BytesWriteError) -> Self {
        PackError {
            value: PackErrorValue::BytesWriteError(error),
        }
    }
}

pub struct ChunkPacketizer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    csid_2_chunk_header: HashMap<u32, ChunkHeader>,
    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    //https://zhuanlan.zhihu.com/p/165976086
    //chunk_info: ChunkInfo,
    max_chunk_size: usize,
    //bytes: Cursor<Vec<u8>>,
    writer: AsyncBytesWriter<S>,
}

impl<S> ChunkPacketizer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io_writer: AsyncBytesWriter<S>) -> Self {
        Self {
            csid_2_chunk_header: HashMap::new(),
            //chunk_info: ChunkInfo::new(),
            writer: io_writer,
            max_chunk_size: 0,
        }
    }
    fn zip_chunk_header(&mut self, chunk_info: &mut ChunkInfo) -> Result<PackResult, PackError> {
        chunk_info.basic_header.format = 0;

        let pre_header = self
            .csid_2_chunk_header
            .get_mut(&chunk_info.basic_header.chunk_stream_id);

        match pre_header {
            Some(val) => {
                let cur_msg_header = &mut chunk_info.message_header;
                let pre_msg_header = &val.message_header;

                if cur_msg_header.msg_streamd_id == pre_msg_header.msg_streamd_id {
                    chunk_info.basic_header.format = 1;
                    cur_msg_header.timestamp -= pre_msg_header.timestamp;

                    if cur_msg_header.msg_type_id == pre_msg_header.msg_type_id
                        && cur_msg_header.msg_length == pre_msg_header.msg_length
                    {
                        chunk_info.basic_header.format = 2;
                        if chunk_info.message_header.timestamp == pre_msg_header.timestamp {
                            chunk_info.basic_header.format = 3;
                        }
                    }
                }
            }

            None => {}
        }

        Ok(PackResult::Success)
    }

    fn write_basic_header(&mut self, fmt: u8, csid: u32) -> Result<(), PackError> {
        if csid >= 64 + 255 {
            self.writer.write_u8(fmt << 6 | 1)?;
            self.writer.write_u16::<BigEndian>((csid - 64) as u16)?;
        } else if csid >= 64 {
            self.writer.write_u8(fmt << 6 | 0)?;
            self.writer.write_u8((csid - 64) as u8)?;
        } else {
            self.writer.write_u8(fmt << 6 | csid as u8)?;
        }

        Ok(())
    }

    fn write_message_header(
        &mut self,
        basic_header: &ChunkBasicHeader,
        message_header: &ChunkMessageHeader,
    ) -> Result<(), PackError> {
        let timestamp = if message_header.timestamp >= 0xFFFFFF {
            0xFFFFFF
        } else {
            message_header.timestamp
        };

        match basic_header.format {
            0 => {
                self.writer.write_u24::<BigEndian>(timestamp)?;
                self.writer
                    .write_u24::<BigEndian>(message_header.msg_length)?;
                self.writer
                    .write_u32::<LittleEndian>(message_header.msg_streamd_id)?;
            }
            1 => {
                self.writer.write_u24::<BigEndian>(timestamp)?;
                self.writer
                    .write_u24::<BigEndian>(message_header.msg_length)?;
            }
            2 => {
                self.writer.write_u24::<BigEndian>(timestamp)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn write_extened_timestamp(&mut self, timestamp: u32) -> Result<(), PackError> {
        self.writer.write_u32::<BigEndian>(timestamp)?;

        Ok(())
    }

    fn write_chunk(&mut self, chunk_info: &mut ChunkInfo) -> Result<(), PackError> {
        self.zip_chunk_header(chunk_info)?;

        let mut whole_payload_size = chunk_info.payload.len();

        self.write_basic_header(
            chunk_info.basic_header.format,
            chunk_info.basic_header.chunk_stream_id,
        )?;
        self.write_message_header(&chunk_info.basic_header, &chunk_info.message_header)?;

        if chunk_info.message_header.is_extended_timestamp {
            self.write_extened_timestamp(chunk_info.message_header.timestamp)?;
        }

        let mut cur_payload_size: usize;

        while whole_payload_size > 0 {
            cur_payload_size = if whole_payload_size > self.max_chunk_size {
                self.max_chunk_size
            } else {
                whole_payload_size
            };

            let payload_bytes = chunk_info.payload.split_to(cur_payload_size);
            self.writer.write(&payload_bytes[0..])?;

            whole_payload_size -= cur_payload_size;

            if whole_payload_size > 0 {
                self.write_basic_header(3, chunk_info.basic_header.chunk_stream_id)?;
                if chunk_info.message_header.is_extended_timestamp {
                    self.write_extened_timestamp(chunk_info.message_header.timestamp)?;
                }
            }
        }

        Ok(())
    }
}
