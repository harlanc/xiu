use bytes::BytesMut;
use chunk::ChunkUnpackError;
use chunk::{ChunkBasicHeader, ChunkHeader, ChunkMessageHeader};
use std::collections::HashMap;

#[derive(Eq, PartialEq, Debug)]
enum UnpackResult {
    ChunkBasicHeaderResult(ChunkBasicHeader),
    ChunkMessageHeaderResult(ChunkMessageHeader),
    Success,
    NotEnoughBytes,
}

enum ChunkParseState {
    Init,
    ParseBasicHeader,
    ParseMessageHeader,
    ParseExtendTimestamp,
    ParsePayload,
}

pub struct ChunkUnpacketizer {
    buffer: BytesMut,
    csid_2_chunk_header: HashMap<u32, ChunkHeader>,
    pub basic_header: ChunkBasicHeader,
}

impl ChunkUnpacketizer {
    pub fn read_chunk(&mut self, bytes: &[u8]) -> Result<UnpackResult, ChunkUnpackError> {
        self.buffer.extend_from_slice(bytes);
        self.read_basic_header()?;

        Ok(UnpackResult::Success)
    }

    pub fn read_bytes(&mut self, bytes_num: usize) -> BytesMut {
        self.buffer.split_to(bytes_num)
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
    pub fn read_basic_header(&mut self) -> Result<UnpackResult, ChunkUnpackError> {
        if self.buffer.len() < 1 {
            return Ok(UnpackResult::NotEnoughBytes);
        }

        let byte = self.read_bytes(1)[0] as u32;

        let format_id = ((byte >> 6) & 0b00000011) as u8;
        let mut csid = byte & 0b00111111;

        match csid {
            0 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.read_bytes(1)[0] as u32;
            }
            1 => {
                if self.buffer.len() < 1 {
                    return Ok(UnpackResult::NotEnoughBytes);
                }
                csid = 64;
                csid += self.read_bytes(1)[0] as u32;
                csid += self.read_bytes(1)[0] as u32 * 256;
            }
            _ => {}
        }

        self.basic_header.chunk_stream_id = csid;
        self.basic_header.format = format_id;

        Ok(UnpackResult::ChunkBasicHeaderResult(ChunkBasicHeader::new(
            format_id, csid,
        )))
    }

    pub fn read_message_header(&mut self) -> Result<UnpackResult, ChunkUnpackError> {
        
        match slf.basic_header.format {
            0 => {}
            1 => {}
            2 => {

            }
        }
        Ok(UnpackResult::Success)
    }
}
