use byteorder::BigEndian;

use bytes::{BufMut, BytesMut};
// use chunk::ChunkUnpackError;
use super::errors::UnpackError;
use super::errors::UnpackErrorValue;
use super::{
    chunk::{ChunkBasicHeader, ChunkInfo, ChunkMessageHeader},
    define::CHUNK_SIZE,
};
use netio::bytes_reader::BytesReader;
use std::{borrow::BorrowMut, cmp::min};

use std::cell::{RefCell, RefMut};
use std::mem;
use std::rc::Rc;

#[derive(Eq, PartialEq, Debug)]
pub enum UnpackResult {
    ChunkBasicHeaderResult(ChunkBasicHeader),
    ChunkMessageHeaderResult(ChunkMessageHeader),
    ChunkInfo(ChunkInfo),
    Success,
    NotEnoughBytes,
    Empty,
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
    Finish,
}

pub struct ChunkUnpacketizer {
    //buffer: BytesMut,
    pub reader: BytesReader,
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
    pub fn new() -> Self {
        Self {
            //buffer: BytesMut::new(),
            reader: BytesReader::new(BytesMut::new()),
            current_chunk_info: ChunkInfo::default(),
            current_read_state: ChunkReadState::Init,
            max_chunk_size: CHUNK_SIZE as usize,
        }
    }

    // fn reader(&mut self) -> RefMut<BytesReader> {
    //     return self.reader.borrow_mut();
    // }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }

    pub fn update_max_chunk_size(&mut self, chunk_size: usize) {
        self.max_chunk_size = chunk_size;
    }

    /******************************************************************************
     * 5.3.1 Chunk Format
     * Each chunk consists of a header and data. The header itself has three parts:
     * +--------------+----------------+--------------------+--------------+
     * | Basic Header | Message Header | Extended Timestamp | Chunk Data |
     * +--------------+----------------+--------------------+--------------+
     * |<------------------- Chunk Header ----------------->|
     ******************************************************************************/
    pub fn read_chunk(&mut self) -> Result<UnpackResult, UnpackError> {
        self.current_read_state = ChunkReadState::ReadBasicHeader;

        let mut result: UnpackResult = UnpackResult::Empty;

        loop {
            result = match self.current_read_state {
                ChunkReadState::ReadBasicHeader => self.read_basic_header()?,
                ChunkReadState::ReadMessageHeader => self.read_message_header()?,
                ChunkReadState::ReadExtendedTimestamp => self.read_extended_timestamp()?,
                ChunkReadState::ReadMessagePayload => self.read_message_payload()?,
                ChunkReadState::Finish => {
                    break;
                }
                _ => {
                    return Err(UnpackError {
                        value: UnpackErrorValue::UnknowReadState,
                    });
                }
            };
        }
        return Ok(result);

        // Ok(UnpackResult::Success)
    }

    /******************************************************************
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
     ***********************************************************************/

    pub fn read_basic_header(&mut self) -> Result<UnpackResult, UnpackError> {
        let byte = self.reader.read_u8()?;

        let format_id = ((byte >> 6) & 0b00000011) as u8;
        let mut csid = (byte & 0b00111111) as u32;

        match csid {
            0 => {
                if self.reader.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.reader.read_u8()? as u32;
            }
            1 => {
                if self.reader.len() < 1 {
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

    pub fn read_message_header(&mut self) -> Result<UnpackResult, UnpackError> {
        match self.current_chunk_info.basic_header.format {
            /*****************************************************************/
            /*      5.3.1.2.1. Type 0                                        */
            /*****************************************************************
             0                   1                   2                   3
             0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            |                timestamp(3bytes)              |message length |
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            | message length (cont)(3bytes) |message type id| msg stream id |
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            |       message stream id (cont) (4bytes)       |
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            *****************************************************************/
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
            /*****************************************************************/
            /*      5.3.1.2.2. Type 1                                        */
            /*****************************************************************
             0                   1                   2                   3
             0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            |                timestamp(3bytes)              |message length |
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            | message length (cont)(3bytes) |message type id|
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            *****************************************************************/
            1 => {
                self.current_message_header().timestamp_delta =
                    self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_length = self.reader.read_u24::<BigEndian>()?;
                self.current_message_header().msg_type_id = self.reader.read_u8()?;

                if self.current_message_header().timestamp_delta >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }
            /************************************************/
            /*      5.3.1.2.3. Type 2                       */
            /************************************************
             0                   1                   2
             0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            |                timestamp(3bytes)              |
            +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            ***************************************************/
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
            let chunkinfo = mem::replace(&mut self.current_chunk_info, ChunkInfo::default());
            self.current_read_state = ChunkReadState::Finish;
            return Ok(UnpackResult::ChunkInfo(chunkinfo));
        }

        self.current_read_state = ChunkReadState::ReadBasicHeader;

        Ok(UnpackResult::Success)
    }
}

#[cfg(test)]
mod tests {

    use super::ChunkInfo;
    use super::ChunkUnpacketizer;
    use super::UnpackResult;
    use bytes::BytesMut;

    #[test]
    fn test_set_chunk_size() {
        let mut unpacker = ChunkUnpacketizer::new();

        let data: [u8; 16] = [
            //
            02, //|format+csid|
            00, 00, 00, //timestamp
            00, 00, 04, //msg_length
            01, //msg_type_id
            00, 00, 00, 00, //msg_stream_id
            00, 00, 10, 00, //body
        ];

        unpacker.extend_data(&data[..]);

        let rv = unpacker.read_chunk();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[00, 00, 10, 00]);

        let expected = ChunkInfo::new(2, 0, 0, 4, 1, 0, body);

        assert_eq!(
            rv.unwrap(),
            UnpackResult::ChunkInfo(expected),
            "not correct"
        )
    }

    #[test]
    fn test_on_connect() {
        // 0000   03 00 00 00 00 00 b1 14 00 00 00 00 02 00 07 63  ...............c
        // 0010   6f 6e 6e 65 63 74 00 3f f0 00 00 00 00 00 00 03  onnect.?........
        // 0020   00 03 61 70 70 02 00 06 68 61 72 6c 61 6e 00 04  ..app...harlan..
        // 0030   74 79 70 65 02 00 0a 6e 6f 6e 70 72 69 76 61 74  type...nonprivat
        // 0040   65 00 08 66 6c 61 73 68 56 65 72 02 00 1f 46 4d  e..flashVer...FM
        // 0050   4c 45 2f 33 2e 30 20 28 63 6f 6d 70 61 74 69 62  LE/3.0 (compatib
        // 0060   6c 65 3b 20 46 4d 53 63 2f 31 2e 30 29 00 06 73  le; FMSc/1.0)..s
        // 0070   77 66 55 72 6c 02 00 1c 72 74 6d 70 3a 2f 2f 6c  wfUrl...rtmp://l
        // 0080   6f 63 61 6c 68 6f 73 74 3a 31 39 33 35 2f 68 61  ocalhost:1935/ha
        // 0090   72 6c 61 6e 00 05 74 63 55 72 6c 02 00 1c 72 74  rlan..tcUrl...rt
        // 00a0   6d 70 3a 2f 2f 6c 6f 63 61 6c 68 6f 73 74 3a 31  mp://localhost:1
        // 00b0   39 33 35 2f 68 61 72 6c 61 6e 00 00 09           935/harlan...
        let data: [u8; 189] = [
            3, //|format+csid|
            0, 0, 0, //timestamp
            0, 0, 177, //msg_length
            20,  //msg_type_id 0x14
            0, 0, 0, 0, //msg_stream_id
            2, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 63, 240, 0, 0, 0, 0, 0, 0, //body
            3, 0, 3, 97, 112, 112, 2, 0, 6, 104, 97, 114, 108, 97, 110, 0, 4, 116, 121, 112, 101,
            2, 0, 10, 110, 111, 110, 112, 114, 105, 118, 97, 116, 101, 0, 8, 102, 108, 97, 115,
            104, 86, 101, 114, 2, 0, 31, 70, 77, 76, 69, 47, 51, 46, 48, 32, 40, 99, 111, 109, 112,
            97, 116, 105, 98, 108, 101, 59, 32, 70, 77, 83, 99, 47, 49, 46, 48, 41, 0, 6, 115, 119,
            102, 85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108,
            104, 111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 5, 116, 99,
            85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104,
            111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 0, 9,
        ];

        let mut unpacker = ChunkUnpacketizer::new();
        unpacker.extend_data(&data[..]);

        let rv = unpacker.read_chunk();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[
            2, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 63, 240, 0, 0, 0, 0, 0, 0, //body
            3, 0, 3, 97, 112, 112, 2, 0, 6, 104, 97, 114, 108, 97, 110, 0, 4, 116, 121, 112, 101,
            2, 0, 10, 110, 111, 110, 112, 114, 105, 118, 97, 116, 101, 0, 8, 102, 108, 97, 115,
            104, 86, 101, 114, 2, 0, 31, 70, 77, 76, 69, 47, 51, 46, 48, 32, 40, 99, 111, 109, 112,
            97, 116, 105, 98, 108, 101, 59, 32, 70, 77, 83, 99, 47, 49, 46, 48, 41, 0, 6, 115, 119,
            102, 85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108,
            104, 111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 5, 116, 99,
            85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104,
            111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 0, 9,
        ]);

        let expected = ChunkInfo::new(3, 0, 0, 177, 20, 0, body);

        assert_eq!(
            rv.unwrap(),
            UnpackResult::ChunkInfo(expected),
            "not correct"
        )
    }
}
