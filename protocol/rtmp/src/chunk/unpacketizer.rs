use byteorder::ByteOrder;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;
use bytes::BytesMut;
// use chunk::ChunkUnpackError;
use chunk::{ChunkBasicHeader, ChunkMessageHeader};
use std::collections::HashMap;
use std::io;
use std::io::Cursor;

#[derive(Eq, PartialEq, Debug)]
pub enum UnpackResult {
    ChunkBasicHeaderResult(ChunkBasicHeader),
    ChunkMessageHeaderResult(ChunkMessageHeader),
    Success,
    NotEnoughBytes,
}

pub enum UnpackErrorValue {
    NotEnoughBytes,
    UnknowReadState,
    IO(io::Error),
}

pub struct UnpackError {
    pub value: UnpackErrorValue,
}

impl From<UnpackErrorValue> for UnpackError {
    fn from(val: UnpackErrorValue) -> Self {
        UnpackError { value: val }
    }
}

impl From<io::Error> for UnpackError {
    fn from(error: io::Error) -> Self {
        UnpackError {
            value: UnpackErrorValue::IO(error),
        }
    }
}

enum ChunkReadState {
    Init,
    ReadBasicHeader,
    ReadMessageHeader,
    ReadExtendedTimestamp,
    ReadMessagePayload,
}

pub struct ChunkInfo {
    pub basic_header: ChunkBasicHeader,
    pub message_header: ChunkMessageHeader,
    pub payload: Bytes,
}
impl ChunkInfo {
    pub fn new() -> ChunkInfo {
        ChunkInfo {
            basic_header: ChunkBasicHeader::new(0, 0),
            message_header: ChunkMessageHeader::new(),
            payload: Bytes::new(),
        }
    }
}

pub struct ChunkUnpacketizer<'a> {
    buffer: BytesMut,
    csid_2_chunk_info: HashMap<u32, ChunkInfo>,
    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    pub current_chunk_info: &'a mut ChunkInfo,
    current_read_state: ChunkReadState,

    // test: HashMap<u32, u32>,
    // pub testval : & 'a mut u32,
}

impl<'a> ChunkUnpacketizer<'a> {
    pub fn read_chunk(&mut self, bytes: &[u8]) -> Result<UnpackResult, UnpackError> {
        self.buffer.extend_from_slice(bytes);
        self.current_read_state = ChunkReadState::ReadBasicHeader;

        loop {
            match self.current_read_state {
                ChunkReadState::ReadBasicHeader => self.read_basic_header()?,
                ChunkReadState::ReadMessageHeader => self.read_message_header()?,
                ChunkReadState::ReadExtendedTimestamp => self.read_extended_timestamp()?,
                ChunkReadState::ReadMessagePayload => self.read_message_payload()?,
                _ => {
                    return Err(UnpackError {
                        value: UnpackErrorValue::UnknowReadState,
                    });
                }
            };
        }

        // Ok(UnpackResult::Success)
    }

    fn read_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, UnpackError> {
        if self.buffer.len() < bytes_num {
            return Err(UnpackError {
                value: UnpackErrorValue::NotEnoughBytes,
            });
        }
        Ok(self.buffer.split_to(bytes_num))
    }

    fn read_bytes_cursor(&mut self, bytes_num: usize) -> Result<Cursor<BytesMut>, UnpackError> {
        let tmp_bytes = self.read_bytes(bytes_num)?;
        let tmp_cursor = Cursor::new(tmp_bytes);
        Ok(tmp_cursor)
    }

    fn read_u8(&mut self) -> Result<u8, UnpackError> {
        let mut cursor = self.read_bytes_cursor(1)?;
        Ok(cursor.read_u8()?)
    }

    fn read_u24<T: ByteOrder>(&mut self) -> Result<u32, UnpackError> {
        let mut cursor = self.read_bytes_cursor(3)?;
        let val = cursor.read_u24::<T>()?;
        Ok(val)
    }

    fn read_u32<T: ByteOrder>(&mut self) -> Result<u32, UnpackError> {
        let mut cursor = self.read_bytes_cursor(4)?;
        let val = cursor.read_u32::<T>()?;

        Ok(val)
    }

    // fn read_u32<>
    /**
     * 5.3.1.1. Chunk Basic Header
     * The Chunk Basic Header encodes the chunk stream ID and the chunk
     * type(represented by fmt field in the figure below). Chunk type
     * determines the format of the encoded message header. Chunk Basic
     * Header field may be 1, 2, or 3 bytes, depending on the chunk stream
     * ID.
     *
     * The bits 0-5 (least significant) in the chunk basic header represent
     * the chunk stream ID.
     *
     * Chunk stream IDs 2-63 can be encoded in the 1-byte version of this
     * field.
     *    0 1 2 3 4 5 6 7
     *   +-+-+-+-+-+-+-+-+
     *   |fmt|   cs id   |
     *   +-+-+-+-+-+-+-+-+
     *   Figure 6 Chunk basic header 1
     *
     * Chunk stream IDs 64-319 can be encoded in the 2-byte version of this
     * field. ID is computed as (the second byte + 64).
     *   0                   1
     *   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
     *   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     *   |fmt|    0      | cs id - 64    |
     *   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     *   Figure 7 Chunk basic header 2
     *
     * Chunk stream IDs 64-65599 can be encoded in the 3-byte version of
     * this field. ID is computed as ((the third byte)*256 + the second byte
     * + 64).
     *    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
     *   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     *   |fmt|     1     |         cs id - 64            |
     *   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     *   Figure 8 Chunk basic header 3
     *
     * cs id: 6 bits
     * fmt: 2 bits
     * cs id - 64: 8 or 16 bits
     *
     * Chunk stream IDs with values 64-319 could be represented by both 2-
     * byte version and 3-byte version of this field.
     */
    #[allow(dead_code)]
    pub fn read_basic_header(&mut self) -> Result<UnpackResult, UnpackError> {
        let byte = self.read_u8()?;

        let format_id = ((byte >> 6) & 0b00000011) as u8;
        let mut csid = (byte & 0b00111111) as u32;

        match csid {
            0 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.read_u8()? as u32;
            }
            1 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.read_u8()? as u32;
                csid += self.read_u8()? as u32 * 256;
            }
            _ => {}
        }

        let csid2 = 32 as u32;

        // test: HashMap<u32, u32>,
        // pub testval : & 'a mut u32,

        // match self.test.get_mut(&csid2) {
        //     Some(val) => {
      
        //         self.testval = val;
        //     }
        //     None => {
        //         self.test.insert(csid2, 0);
        //     }
        // }

        match self.csid_2_chunk_info.get_mut(&csid2) {
            Some(chunk_info) => {
                let aa  = chunk_info;
                self.current_chunk_info = aa;
            }
            None => {
                self.csid_2_chunk_info.insert(csid, ChunkInfo::new());
            }
        }

        self.current_chunk_info.basic_header.chunk_stream_id = csid;
        self.current_chunk_info.basic_header.format = format_id;

        self.current_read_state = ChunkReadState::ReadMessageHeader;

        Ok(UnpackResult::ChunkBasicHeaderResult(ChunkBasicHeader::new(
            format_id, csid,
        )))
    }

    fn current_message_header(&mut self) -> &mut ChunkMessageHeader {
        &mut self.current_chunk_info.message_header
    }

    #[allow(dead_code)]
    pub fn read_message_header(&mut self) -> Result<UnpackResult, UnpackError> {
        match self.current_chunk_info.basic_header.format {
            0 => {
                // let mut val = self.read_bytes(11);
                self.current_message_header().timestamp = self.read_u24::<BigEndian>()?;
                self.current_message_header().msg_length = self.read_u24::<BigEndian>()?;
                self.current_message_header().msg_type_id = self.read_u8()?;
                self.current_message_header().msg_streamd_id = self.read_u32::<BigEndian>()?;

                if self.current_message_header().timestamp >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            1 => {
                self.current_message_header().timestamp_delta = self.read_u24::<BigEndian>()?;
                self.current_message_header().msg_length = self.read_u24::<BigEndian>()?;
                self.current_message_header().msg_type_id = self.read_u8()?;

                if self.current_message_header().timestamp_delta >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            2 => {
                self.current_message_header().timestamp_delta = self.read_u24::<BigEndian>()?;

                if self.current_message_header().timestamp_delta >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            _ => {}
        }

        self.current_read_state = ChunkReadState::ReadExtendedTimestamp;

        Ok(UnpackResult::Success)
    }
    #[allow(dead_code)]
    pub fn read_extended_timestamp(&mut self) -> Result<UnpackResult, UnpackError> {
        let mut extended_timestamp: u32 = 0;

        if self.current_message_header().is_extended_timestamp {
            extended_timestamp = self.read_u32::<BigEndian>()?;
        }

        match self.current_chunk_info.basic_header.format {
            0 => {
                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp = extended_timestamp;
                }
            }
            1 => {
                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp += extended_timestamp;
                } else {
                    self.current_message_header().timestamp +=
                        self.current_message_header().timestamp_delta;
                }
            }
            2 => {
                self.current_message_header().timestamp_delta = self.read_u24::<BigEndian>()?;

                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp += extended_timestamp;
                } else {
                    self.current_message_header().timestamp +=
                        self.current_message_header().timestamp_delta;
                }
            }
            _ => {}
        }

        self.current_read_state = ChunkReadState::ReadMessagePayload;

        Ok(UnpackResult::Success)
    }

    pub fn read_message_payload(&mut self) -> Result<UnpackResult, UnpackError> {
        Ok(UnpackResult::Success)
    }
}
