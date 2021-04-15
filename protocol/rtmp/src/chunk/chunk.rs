use bytes::BytesMut;

//5.3.1.1
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ChunkBasicHeader {
    pub format: u8,
    pub chunk_stream_id: u32,
}

impl ChunkBasicHeader {
    pub fn new(fmt: u8, csid: u32) -> ChunkBasicHeader {
        ChunkBasicHeader {
            format: fmt,
            chunk_stream_id: csid,
        }
    }
}

//5.3.1.2
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ChunkMessageHeader {
    pub timestamp: u32,
    pub msg_length: u32,
    pub msg_type_id: u8,
    pub msg_streamd_id: u32,
    pub timestamp_delta: u32,
    pub is_extended_timestamp: bool,
}

impl ChunkMessageHeader {
    pub fn new(timestamp: u32, msg_length: u32, msg_type_id: u8, msg_stream_id: u32) -> Self {
        Self {
            timestamp: timestamp,
            msg_length: msg_length,
            msg_type_id: msg_type_id,
            msg_streamd_id: msg_stream_id,
            timestamp_delta: 0,
            is_extended_timestamp: false,
        }
    }
}

pub struct ChunkHeader {
    pub basic_header: ChunkBasicHeader,
    pub message_header: ChunkMessageHeader,
}

impl ChunkHeader {
    pub fn new() -> ChunkHeader {
        ChunkHeader {
            basic_header: ChunkBasicHeader::new(0, 0),
            message_header: ChunkMessageHeader::new(0, 0, 0, 0),
        }
    }
}

// pub struct Chunk {
//     basic_header: ChunkBasicHeader,
//     message_header: ChunkMessageHeader,
//     raw_data: BytesMut,
// }

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ChunkInfo {
    pub basic_header: ChunkBasicHeader,
    pub message_header: ChunkMessageHeader,
    pub payload: BytesMut,
}
impl ChunkInfo {
    pub fn new(
        csid: u32,
        format: u8,
        timestamp: u32,
        msg_length: u32,
        msg_type_id: u8,
        msg_stream_id: u32,
        payload: BytesMut,
    ) -> Self {
        Self {
            basic_header: ChunkBasicHeader::new(format, csid),
            message_header: ChunkMessageHeader::new(
                timestamp,
                msg_length,
                msg_type_id,
                msg_stream_id,
            ),
            payload: payload,
        }
    }

    pub fn default() -> ChunkInfo {
        ChunkInfo::new(0, 0, 0, 0, 0, 0, BytesMut::new())
    }
}

// impl Chunk {
//     pub fn chunk_read(&mut self, bytes: &[u8]) -> Result {
//         self.buffer.extend_from_slice(bytes);
//     }

//     pub fn read_basic_header(&mut self, bytes: &[u8]) -> Result<UnpackResult, ChunkUnpackError> {
//         if self.buffer.len() < 1 {
//             return Ok(UnpackResult::NotEnoughBytes);
//         }
//     }
// }
