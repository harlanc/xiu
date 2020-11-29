use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};

use chunk::{ChunkBasicHeader, ChunkHeader, ChunkInfo, ChunkMessageHeader};
use std::collections::HashMap;
use std::io;
use std::io::{Cursor, Write};

#[derive(Eq, PartialEq, Debug)]
pub enum PackResult {
    Success,
    NotEnoughBytes,
}

pub enum PackErrorValue {
    NotExistHeader,
    UnknowReadState,
    IO(io::Error),
}

pub struct PackError {
    pub value: PackErrorValue,
}

impl From<PackErrorValue> for PackError {
    fn from(val: PackErrorValue) -> Self {
        PackError { value: val }
    }
}

impl From<io::Error> for PackError {
    fn from(error: io::Error) -> Self {
        PackError {
            value: PackErrorValue::IO(error),
        }
    }
}

pub struct ChunkPacketizer {
    csid_2_chunk_header: HashMap<u32, ChunkHeader>,
    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    //https://zhuanlan.zhihu.com/p/165976086
    chunk_info: ChunkInfo,
    max_chunk_size: usize,
    bytes: Cursor<Vec<u8>>,
}

impl ChunkPacketizer {
    fn zip_chunk_header(self, chunk_info: &ChunkInfo) -> Result<PackResult, PackError> {
        // let mut buffer =  Cursor::new(Vec::new());

        // let mut bytes = Cursor::new(Vec::new());

        chunk_info.basic_header.format = 0;

        let pre_header = self
            .csid_2_chunk_header
            .get_mut(&chunk_info.basic_header.chunk_stream_id);

        match pre_header {
            Some(val) => {
                let cur_msg_header = &chunk_info.message_header;
                let pre_msg_header = &val.message_header;

                if cur_msg_header.msg_streamd_id == pre_msg_header.msg_streamd_id {
                    chunk_info.basic_header.format = 1;
                    chunk_info.message_header.timestamp -= pre_msg_header.timestamp;

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

    fn write_u8(&mut self, byte: u8) -> Result<(), PackError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    fn write_u16(&mut self, bytes: u16) -> Result<(), PackError> {
        self.bytes.write_u16::<BigEndian>(bytes)?;
        Ok(())
    }

    fn write_u24(&mut self, bytes: u32) -> Result<(), PackError> {
        self.bytes.write_u24::<BigEndian>(bytes)?;
        Ok(())
    }

    fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), PackError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), PackError> {
        self.bytes.write(buf)?;
        Ok(())
    }

    fn write_basic_header(&mut self, fmt: u8, csid: u32) -> Result<(), PackError> {
        if csid >= 64 + 255 {
            self.write_u8(fmt << 6 | 1)?;
            self.write_u16((csid - 64) as u16)?;
        } else if csid >= 64 {
            self.write_u8(fmt << 6 | 0)?;
            self.write_u8((csid - 64) as u8)?;
        } else {
            self.write_u8(fmt << 6 | csid as u8)?;
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
                self.write_u24(timestamp)?;
                self.write_u24(message_header.msg_length)?;
                self.write_u32::<LittleEndian>(message_header.msg_streamd_id)?;
            }
            1 => {
                self.write_u24(timestamp)?;
                self.write_u24(message_header.msg_length)?;
            }
            2 => {
                self.write_u24(timestamp)?;
            }
            3 => {}
        }

        Ok(())
    }

    fn write_extened_timestamp(&mut self, timestamp: u32) -> Result<(), PackError> {
        self.write_u32::<BigEndian>(timestamp)?;

        Ok(())
    }

    fn write_chunk(self, chunk_info: &ChunkInfo) -> Result<(), PackError> {
        self.zip_chunk_header(chunk_info)?;

        let whole_payload_size = chunk_info.payload.len();

        self.write_basic_header(
            chunk_info.basic_header.format,
            chunk_info.basic_header.chunk_stream_id,
        )?;
        self.write_message_header(&chunk_info.basic_header, &chunk_info.message_header)?;

        if chunk_info.message_header.is_extended_timestamp {
            self.write_extened_timestamp(chunk_info.message_header.timestamp)?;
        }

        let cur_payload_size: usize;

        while whole_payload_size > 0 {
            cur_payload_size = if whole_payload_size > self.max_chunk_size {
                self.max_chunk_size
            } else {
                whole_payload_size
            };

            let payload_bytes = chunk_info.payload.split_to(cur_payload_size);
            self.write(&payload_bytes[0..])?;

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
