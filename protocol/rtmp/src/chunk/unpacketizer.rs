use {
    super::{
        chunk::{ChunkBasicHeader, ChunkHeader, ChunkInfo, ChunkMessageHeader},
        define,
        errors::{UnpackError, UnpackErrorValue},
    },
    crate::messages::define::msg_type_id,
    byteorder::{BigEndian, LittleEndian},
    bytes::{BufMut, BytesMut},
    bytesio::bytes_reader::BytesReader,
    chrono::prelude::*,
    std::{cmp::min, collections::HashMap, vec::Vec},
};

#[derive(Eq, PartialEq, Debug)]
pub enum UnpackResult {
    ChunkBasicHeaderResult(ChunkBasicHeader),
    ChunkMessageHeaderResult(ChunkMessageHeader),
    ChunkInfo(ChunkInfo),
    Chunks(Vec<ChunkInfo>),
    Success,
    NotEnoughBytes,
    Empty,
}

#[derive(Copy, Clone)]
enum ChunkReadState {
    ReadBasicHeader = 1,
    ReadMessageHeader = 2,
    ReadExtendedTimestamp = 3,
    ReadMessagePayload = 4,
    Finish = 5,
}

#[derive(Copy, Clone)]
enum MessageHeaderReadState {
    ReadTimeStamp = 1,
    ReadMsgLength = 2,
    ReadMsgTypeID = 3,
    ReadMsgStreamID = 4,
}

fn f(chunk: &ChunkReadState) -> u8 {
    *chunk as u8
}

pub struct ChunkUnpacketizer {
    pub reader: BytesReader,

    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    //https://zhuanlan.zhihu.com/p/165976086
    pub current_chunk_info: ChunkInfo,
    chunk_headers: HashMap<u32, ChunkHeader>,
    chunk_read_state: ChunkReadState,
    msg_header_read_state: MessageHeaderReadState,
    max_chunk_size: usize,
    chunk_index: u32,
    pub session_type: u8,
}

impl ChunkUnpacketizer {
    pub fn new() -> Self {
        Self {
            reader: BytesReader::new(BytesMut::new()),
            current_chunk_info: ChunkInfo::default(),
            chunk_headers: HashMap::new(),
            chunk_read_state: ChunkReadState::ReadBasicHeader,
            msg_header_read_state: MessageHeaderReadState::ReadTimeStamp,
            max_chunk_size: define::INIT_CHUNK_SIZE as usize,
            chunk_index: 0,
            session_type: 0,
        }
    }

    pub fn extend_data(&mut self, data: &[u8]) {
        self.reader.extend_from_slice(data);
    }

    pub fn update_max_chunk_size(&mut self, chunk_size: usize) {
        self.max_chunk_size = chunk_size;
    }

    pub fn read_chunks(&mut self) -> Result<UnpackResult, UnpackError> {
        log::trace!(
            "read chunks begin, current time: {}, and read state: {}",
            Local::now().timestamp_nanos(),
            f(&self.chunk_read_state)
        );

        // log::trace!(
        //     "read chunks, reader remaining data: {}",
        //     self.reader.get_remaining_bytes()
        // );

        let mut chunks: Vec<ChunkInfo> = Vec::new();

        loop {
            match self.read_chunk() {
                Ok(chunk) => match chunk {
                    UnpackResult::ChunkInfo(chunk_info) => {
                        let msg_type_id = chunk_info.message_header.msg_type_id.clone();
                        chunks.push(chunk_info);

                        //if the chunk_size is changed, then break and update chunk_size
                        if msg_type_id == msg_type_id::SET_CHUNK_SIZE {
                            break;
                        }
                    }
                    _ => continue,
                },
                Err(_) => break,
            }
        }

        log::trace!(
            "read chunks end, current time: {}, and read state: {}",
            Local::now().timestamp_nanos(),
            f(&self.chunk_read_state)
        );

        if chunks.len() > 0 {
            return Ok(UnpackResult::Chunks(chunks));
        } else {
            return Err(UnpackError {
                value: UnpackErrorValue::EmptyChunks,
            });
        }
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
        let mut result: UnpackResult = UnpackResult::Empty;

        log::trace!(
            "read chunk begin, current time: {}, and read state: {}, and chunk index: {}",
            Local::now().timestamp_nanos(),
            f(&self.chunk_read_state),
            self.chunk_index,
        );

        self.chunk_index = self.chunk_index + 1;

        loop {
            result = match self.chunk_read_state {
                ChunkReadState::ReadBasicHeader => self.read_basic_header()?,
                ChunkReadState::ReadMessageHeader => self.read_message_header()?,
                ChunkReadState::ReadExtendedTimestamp => self.read_extended_timestamp()?,
                ChunkReadState::ReadMessagePayload => self.read_message_payload()?,
                ChunkReadState::Finish => {
                    self.chunk_read_state = ChunkReadState::ReadBasicHeader;
                    break;
                }
            };
        }

        log::trace!(
            "read chunk end, current time: {}, and read state: {}, and chunk index: {}",
            Local::now().timestamp_nanos(),
            f(&self.chunk_read_state),
            self.chunk_index,
        );
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

        //todo
        if csid != self.current_chunk_info.basic_header.chunk_stream_id {
            if let Some(header) = self.chunk_headers.get_mut(&csid) {
                self.current_chunk_info.basic_header = header.basic_header.clone();
                self.current_chunk_info.message_header = header.message_header.clone();
            }
        }

        self.current_chunk_info.basic_header.chunk_stream_id = csid;
        self.current_chunk_info.basic_header.format = format_id;

        self.chunk_read_state = ChunkReadState::ReadMessageHeader;

        Ok(UnpackResult::ChunkBasicHeaderResult(ChunkBasicHeader::new(
            format_id, csid,
        )))
    }

    fn current_message_header(&mut self) -> &mut ChunkMessageHeader {
        &mut self.current_chunk_info.message_header
    }

    pub fn read_message_header(&mut self) -> Result<UnpackResult, UnpackError> {
        log::trace!(
            "read_message_header, left bytes length: {}",
            self.reader.len(),
        );

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
                loop {
                    match self.msg_header_read_state {
                        MessageHeaderReadState::ReadTimeStamp => {
                            self.current_message_header().timestamp =
                                self.reader.read_u24::<BigEndian>()?;
                            self.msg_header_read_state = MessageHeaderReadState::ReadMsgLength;
                        }
                        MessageHeaderReadState::ReadMsgLength => {
                            self.current_message_header().msg_length =
                                self.reader.read_u24::<BigEndian>()?;

                            log::trace!(
                                "read_message_header format 0, msg_length: {}",
                                self.current_message_header().msg_length,
                            );
                            self.msg_header_read_state = MessageHeaderReadState::ReadMsgTypeID;
                        }
                        MessageHeaderReadState::ReadMsgTypeID => {
                            self.current_message_header().msg_type_id = self.reader.read_u8()?;

                            log::trace!(
                                "read_message_header format 0, msg_type_id: {}",
                                self.current_message_header().msg_type_id
                            );
                            self.msg_header_read_state = MessageHeaderReadState::ReadMsgStreamID;
                        }
                        MessageHeaderReadState::ReadMsgStreamID => {
                            self.current_message_header().msg_streamd_id =
                                self.reader.read_u32::<LittleEndian>()?;
                            self.msg_header_read_state = MessageHeaderReadState::ReadTimeStamp;
                            break;
                        }
                    }
                }

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
                loop {
                    match self.msg_header_read_state {
                        MessageHeaderReadState::ReadTimeStamp => {
                            self.current_message_header().timestamp_delta =
                                self.reader.read_u24::<BigEndian>()?;
                            self.msg_header_read_state = MessageHeaderReadState::ReadMsgLength;
                        }
                        MessageHeaderReadState::ReadMsgLength => {
                            self.current_message_header().msg_length =
                                self.reader.read_u24::<BigEndian>()?;

                            log::trace!(
                                "read_message_header format 1, msg_length: {}",
                                self.current_message_header().msg_length
                            );
                            self.msg_header_read_state = MessageHeaderReadState::ReadMsgTypeID;
                        }
                        MessageHeaderReadState::ReadMsgTypeID => {
                            self.current_message_header().msg_type_id = self.reader.read_u8()?;

                            log::trace!(
                                "read_message_header format 1, msg_type_id: {}",
                                self.current_message_header().msg_type_id
                            );
                            self.msg_header_read_state = MessageHeaderReadState::ReadTimeStamp;
                            break;
                        }
                        _ => {
                            log::error!("error happend when read chunk message header");
                            break;
                        }
                    }
                }

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
                log::trace!(
                    "read_message_header format 2, msg_type_id: {}",
                    self.current_message_header().msg_type_id
                );
                self.current_message_header().timestamp_delta =
                    self.reader.read_u24::<BigEndian>()?;

                if self.current_message_header().timestamp_delta >= 0xFFFFFF {
                    self.current_message_header().is_extended_timestamp = true;
                }
            }

            _ => {}
        }

        self.chunk_read_state = ChunkReadState::ReadExtendedTimestamp;

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
                if self.current_message_header().is_extended_timestamp {
                    self.current_message_header().timestamp =
                        self.current_message_header().timestamp - 0xFFFFFF + extended_timestamp;
                } else {
                    self.current_message_header().timestamp +=
                        self.current_message_header().timestamp_delta;
                }
            }
            //todo: 3 should also be processed
            _ => {}
        }

        self.chunk_read_state = ChunkReadState::ReadMessagePayload;

        Ok(UnpackResult::Success)
    }

    pub fn read_message_payload(&mut self) -> Result<UnpackResult, UnpackError> {
        let whole_msg_length = self.current_message_header().msg_length as usize;
        let remaining_bytes = whole_msg_length - self.current_chunk_info.payload.len();

        log::trace!(
            "read_message_payload whole msg length: {} and remaining bytes: {}",
            whole_msg_length,
            remaining_bytes
        );

        let mut need_read_length = remaining_bytes;
        if whole_msg_length > self.max_chunk_size {
            need_read_length = min(remaining_bytes, self.max_chunk_size);
        }

        let remaining_mut = self.current_chunk_info.payload.remaining_mut();
        if need_read_length > remaining_mut {
            let additional = need_read_length - remaining_mut;
            self.current_chunk_info.payload.reserve(additional);
        }

        log::trace!("read_message_payload buffer len:{}", self.reader.len());

        let payload_data = self.reader.read_bytes(need_read_length)?;
        self.current_chunk_info
            .payload
            .extend_from_slice(&payload_data[..]);

        log::trace!(
            "read_message_payload current msg payload len:{}",
            self.current_chunk_info.payload.len()
        );

        if self.current_chunk_info.payload.len() == whole_msg_length {
            self.chunk_read_state = ChunkReadState::Finish;
            let chunk_info = self.current_chunk_info.clone();
            self.current_chunk_info.payload.clear();

            let csid = self.current_chunk_info.basic_header.chunk_stream_id;

            //todo
            if let Some(header) = self.chunk_headers.get_mut(&csid) {
                header.basic_header = self.current_chunk_info.basic_header.clone();
                header.message_header = self.current_chunk_info.message_header.clone();
            } else {
                let chunk_header = ChunkHeader {
                    basic_header: self.current_chunk_info.basic_header.clone(),
                    message_header: self.current_chunk_info.message_header.clone(),
                };
                self.chunk_headers.insert(csid, chunk_header);
            }

            // self.chunk_headers
            //     .entry(self.current_chunk_info.basic_header.chunk_stream_id)
            //     .or_insert(chunk_header);

            return Ok(UnpackResult::ChunkInfo(chunk_info));
        }

        self.chunk_read_state = ChunkReadState::ReadBasicHeader;

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

    // #[test]
    // fn test_window_acknowlage_size_set_peer_bandwidth() {
    //     let mut unpacker = ChunkUnpacketizer::new();

    //     let data: [u8; 33] = [
    //         0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //         0x10, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x10, 0x00, 0x02,
    //     ];

    //     unpacker.extend_data(&data[..]);

    //     let rv = unpacker.read_chunk();

    //     let rv2 = unpacker.read_chunk();

    //     let mut body = BytesMut::new();
    //     body.extend_from_slice(&[00, 00, 10, 00]);

    //     let expected = ChunkInfo::new(2, 0, 0, 4, 1, 0, body);

    //     assert_eq!(
    //         rv.unwrap(),
    //         UnpackResult::ChunkInfo(expected),
    //         "not correct"
    //     )
    // }

    // #[test]
    // fn test_on_connect() {
    //     // 0000   03 00 00 00 00 00 b1 14 00 00 00 00 02 00 07 63  ...............c
    //     // 0010   6f 6e 6e 65 63 74 00 3f f0 00 00 00 00 00 00 03  onnect.?........
    //     // 0020   00 03 61 70 70 02 00 06 68 61 72 6c 61 6e 00 04  ..app...harlan..
    //     // 0030   74 79 70 65 02 00 0a 6e 6f 6e 70 72 69 76 61 74  type...nonprivat
    //     // 0040   65 00 08 66 6c 61 73 68 56 65 72 02 00 1f 46 4d  e..flashVer...FM
    //     // 0050   4c 45 2f 33 2e 30 20 28 63 6f 6d 70 61 74 69 62  LE/3.0 (compatib
    //     // 0060   6c 65 3b 20 46 4d 53 63 2f 31 2e 30 29 00 06 73  le; FMSc/1.0)..s
    //     // 0070   77 66 55 72 6c 02 00 1c 72 74 6d 70 3a 2f 2f 6c  wfUrl...rtmp://l
    //     // 0080   6f 63 61 6c 68 6f 73 74 3a 31 39 33 35 2f 68 61  ocalhost:1935/ha
    //     // 0090   72 6c 61 6e 00 05 74 63 55 72 6c 02 00 1c 72 74  rlan..tcUrl...rt
    //     // 00a0   6d 70 3a 2f 2f 6c 6f 63 61 6c 68 6f 73 74 3a 31  mp://localhost:1
    //     // 00b0   39 33 35 2f 68 61 72 6c 61 6e 00 00 09           935/harlan...
    //     // let data: [u8; 189] = [
    //     //     3, //|format+csid|
    //     //     0, 0, 0, //timestamp
    //     //     0, 0, 177, //msg_length
    //     //     20,  //msg_type_id 0x14
    //     //     0, 0, 0, 0, //msg_stream_id
    //     //     2, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 63, 240, 0, 0, 0, 0, 0, 0, //body
    //     //     3, 0, 3, 97, 112, 112, 2, 0, 6, 104, 97, 114, 108, 97, 110, 0, 4, 116, 121, 112, 101,
    //     //     2, 0, 10, 110, 111, 110, 112, 114, 105, 118, 97, 116, 101, 0, 8, 102, 108, 97, 115,
    //     //     104, 86, 101, 114, 2, 0, 31, 70, 77, 76, 69, 47, 51, 46, 48, 32, 40, 99, 111, 109, 112,
    //     //     97, 116, 105, 98, 108, 101, 59, 32, 70, 77, 83, 99, 47, 49, 46, 48, 41, 0, 6, 115, 119,
    //     //     102, 85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108,
    //     //     104, 111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 5, 116, 99,
    //     //     85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104,
    //     //     111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 0, 9,
    //     // ];

    //     let data: [u8; 189] = [
    //         0x03,
    //         0x00, 0x00, 0x00,
    //         0x00, 0x00, 0xb1,
    //         0x14,
    //         0x00, 0x00, 0x00, 0x00,
    //         0x02, 0x00,
    //         0x07, 0x63, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, 0x00, 0x3f, 0xf0, 0x00, 0x00, 0x00,
    //         0x00, 0x00, 0x00, 0x03, 0x00, 0x03, 0x61, 0x70, 0x70, 0x02, 0x00, 0x06, 0x68, 0x61,
    //         0x72, 0x6c, 0x61, 0x6e, 0x00, 0x04, 0x74, 0x79, 0x70, 0x65, 0x02, 0x00, 0x0a, 0x6e,
    //         0x6f, 0x6e, 0x70, 0x72, 0x69, 0x76, 0x61, 0x74, 0x65, 0x00, 0x08, 0x66, 0x6c, 0x61,
    //         0x73, 0x68, 0x56, 0x65, 0x72, 0x02, 0x00, 0x1f, 0x46, 0x4d, 0x4c, 0x45, 0x2f, 0x33,
    //         0x2e, 0x30, 0x20, 0x28, 0x63, 0x6f, 0x6d, 0x70, 0x61, 0x74, 0x69, 0x62, 0x6c, 0x65,
    //         0x3b, 0x20, 0x46, 0x4d, 0x53, 0x63, 0x2f, 0x31, 0x2e, 0x30, 0x29, 0x00, 0x06, 0x73,
    //         0x77, 0x66, 0x55, 0x72, 0x6c, 0x02, 0x00, 0x1c, 0x72, 0x74, 0x6d, 0x70, 0x3a, 0x2f,
    //         0x2f, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68, 0x6f, 0x73, 0x74, 0x3a, 0x31, 0x39, 0x33,
    //         0x35, 0x2f, 0x68, 0x61, 0x72, 0x6c, 0x61, 0x6e, 0x00, 0x05, 0x74, 0x63, 0x55, 0x72,
    //         0x6c, 0x02, 0x00, 0x1c, 0x72, 0x74, 0x6d, 0x70, 0x3a, 0x2f, 0x2f, 0x6c, 0x6f, 0x63,
    //         0x61, 0x6c, 0x68, 0x6f, 0x73, 0x74, 0x3a, 0x31, 0x39, 0x33, 0x35, 0x2f, 0x68, 0x61,
    //         0x72, 0x6c, 0x61, 0x6e, 0x00, 0x00, 0x09,
    //     ];

    //     let mut unpacker = ChunkUnpacketizer::new();
    //     unpacker.extend_data(&data[..]);

    //     let rv = unpacker.read_chunk();
    //     match &rv {
    //         Err(err) => {
    //             println!("==={}===", err);
    //         }
    //         _ => {}
    //     }

    //     let mut body = BytesMut::new();
    //     body.extend_from_slice(&[
    //         2, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 63, 240, 0, 0, 0, 0, 0, 0, //body
    //         3, 0, 3, 97, 112, 112, 2, 0, 6, 104, 97, 114, 108, 97, 110, 0, 4, 116, 121, 112, 101,
    //         2, 0, 10, 110, 111, 110, 112, 114, 105, 118, 97, 116, 101, 0, 8, 102, 108, 97, 115,
    //         104, 86, 101, 114, 2, 0, 31, 70, 77, 76, 69, 47, 51, 46, 48, 32, 40, 99, 111, 109, 112,
    //         97, 116, 105, 98, 108, 101, 59, 32, 70, 77, 83, 99, 47, 49, 46, 48, 41, 0, 6, 115, 119,
    //         102, 85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108,
    //         104, 111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 5, 116, 99,
    //         85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104,
    //         111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 0, 9,
    //     ]);

    //     let expected = ChunkInfo::new(3, 0, 0, 177, 20, 0, body);

    //     assert_eq!(
    //         rv.unwrap(),
    //         UnpackResult::ChunkInfo(expected),
    //         "not correct"
    //     )
    // }
}
