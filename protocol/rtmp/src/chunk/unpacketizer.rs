use byteorder::{BigEndian, ReadBytesExt};
//use bytes::Bytes;
use bytes::{BufMut, BytesMut};
// use chunk::ChunkUnpackError;
use super::chunk::{ChunkBasicHeader, ChunkInfo, ChunkMessageHeader};
use liverust_lib::netio::reader::{IOReadError, Reader};
use std::cmp::min;
use std::collections::HashMap;
use std::mem;

use crate::netconnection;

#[derive(Eq, PartialEq, Debug)]
pub enum UnpackResult {
    ChunkBasicHeaderResult(ChunkBasicHeader),
    ChunkMessageHeaderResult(ChunkMessageHeader),
    ChunkInfo(ChunkInfo),
    Success,
    NotEnoughBytes,
}

pub enum UnpackErrorValue {
    IO(IOReadError),
    UnknowReadState,
    //IO(io::Error),
}

pub struct UnpackError {
    pub value: UnpackErrorValue,
}

impl From<UnpackErrorValue> for UnpackError {
    fn from(val: UnpackErrorValue) -> Self {
        UnpackError { value: val }
    }
}

impl From<IOReadError> for UnpackError {
    fn from(error: IOReadError) -> Self {
        UnpackError {
            value: UnpackErrorValue::IO(error),
        }
    }
}

// impl From<IOReadErrorValue> for UnpackError {
//     fn from(error: IOReadErrorValue) -> Self {
//         UnpackError {
//             value: UnpackErrorValue::IOReadErrorValue(error),
//         }
//     }
// }

enum ChunkReadState {
    Init,
    ReadBasicHeader,
    ReadMessageHeader,
    ReadExtendedTimestamp,
    ReadMessagePayload,
}

pub struct ChunkUnpacketizer {
    buffer: BytesMut,
    reader: Reader,
    //reader :
    //: HashMap<u32, ChunkInfo>,
    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    //https://zhuanlan.zhihu.com/p/165976086
    pub current_chunk_info: ChunkInfo,
    current_read_state: ChunkReadState,
    max_chunk_size: usize,
    // test: HashMap<u32, u32>,
    // pub testval : & 'a mut u32,
}

impl ChunkUnpacketizer {
    pub fn new(input: BytesMut) -> ChunkUnpacketizer {
        ChunkUnpacketizer {
            buffer: BytesMut::new(),
            reader: Reader::new(input),
            current_chunk_info: ChunkInfo::new(),
            current_read_state: ChunkReadState::Init,
            max_chunk_size: 0,
        }
    }
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
        let byte = self.reader.read_u8()?;

        let format_id = ((byte >> 6) & 0b00000011) as u8;
        let mut csid = (byte & 0b00111111) as u32;

        match csid {
            0 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.reader.read_u8()? as u32;
            }
            1 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.reader.read_u8()? as u32;
                csid += self.reader.read_u8()? as u32 * 256;
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

        // match self.csid_2_chunk_info.get_mut(&csid2) {
        //     Some(chunk_info) => {
        //         let aa = chunk_info;
        //         self.current_chunk_info = aa;
        //     }
        //     None => {
        //         self.csid_2_chunk_info.insert(csid, ChunkInfo::new());
        //     }
        // }

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
                self.current_message_header().timestamp = self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_length = self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_type_id = self.reader.read_u8()?;
                self.current_message_header().msg_streamd_id =
                    self.reader.read_u32::<BigEndian>()?;

                if self.current_message_header().timestamp >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            1 => {
                self.current_message_header().timestamp_delta =
                    self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_length = self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_type_id = self.reader.read_u8()?;

                if self.current_message_header().timestamp_delta >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            2 => {
                self.current_message_header().timestamp_delta =
                    self.reader.read_u24::<BigEndian>()?;

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
            extended_timestamp = self.reader.read_u32::<BigEndian>()?;
        }

        match self.current_chunk_info.basic_header.format {
            0 => {
                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp = extended_timestamp;
                }
            }
            1 => {
                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp =
                        self.current_message_header().timestamp - 0xFFFFFF + extended_timestamp;
                } else {
                    self.current_message_header().timestamp +=
                        self.current_message_header().timestamp_delta;
                }
            }
            2 => {
                //self.current_message_header().timestamp_delta = self.read_u24::<BigEndian>()?;

                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp =
                        self.current_message_header().timestamp - 0xFFFFFF + extended_timestamp;
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
        let mut whole_msg_length = self.current_message_header().msg_length as usize;
        let remaining_bytes = whole_msg_length - self.current_chunk_info.payload.len();

        let mut need_read_length = remaining_bytes;
        if whole_msg_length > self.max_chunk_size {
            need_read_length = min(remaining_bytes, self.max_chunk_size);
        }

        let remaining_mut = self.current_chunk_info.payload.remaining_mut();
        if need_read_length > remaining_mut {
            let additional = need_read_length - remaining_mut;
            self.current_chunk_info.payload.reserve(additional);
        }

        let payload_data = self.reader.read_bytes(need_read_length)?;
        self.current_chunk_info
            .payload
            .extend_from_slice(&payload_data[..]);

        if self.current_chunk_info.payload.len() == whole_msg_length {
            let chunkinfo = mem::replace(&mut self.current_chunk_info, ChunkInfo::new());
            return Ok(UnpackResult::ChunkInfo(chunkinfo));
        }

        self.current_read_state = ChunkReadState::ReadBasicHeader;

        Ok(UnpackResult::Success)
    }
}
