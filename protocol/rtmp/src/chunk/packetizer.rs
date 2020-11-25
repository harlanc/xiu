use bytes::BytesMut;
use chunk::{ChunkHeader, ChunkInfo};
use std::collections::HashMap;

#[derive(Eq, PartialEq, Debug)]
pub enum PackResult {
    Success,
    NotEnoughBytes,
}

pub enum PackError {
    NotExistHeader,
    UnknowReadState,
}

pub struct ChunkPacketizer {
    buffer: BytesMut,
    csid_2_chunk_header: HashMap<u32, ChunkHeader>,
    //https://doc.rust-lang.org/stable/rust-by-example/scope/lifetime/fn.html
    //https://zhuanlan.zhihu.com/p/165976086
    chunk_info: ChunkInfo,
    max_chunk_size: usize,
}

impl ChunkPacketizer {
    fn zip_chunk_header(self, chunk_info: &ChunkInfo) -> Result<PackResult, PackError> {
        let pre_header = match self
            .csid_2_chunk_header
            .get_mut(&chunk_info.basic_header.chunk_stream_id)
        {
            Some(val) => val,
            None => return Err(PackError::NotExistHeader),
        };

        let format u8;

        let cur_msg_header = &chunk_info.message_header;
        let pre_msg_header = &pre_header.message_header;

        if 0 != pre_header.basic_header.format
            && cur_msg_header.timestamp >= pre_msg_header.timestamp
            && cur_msg_header.timestamp - pre_msg_header.timestamp < 0xFFFFFF
            && cur_msg_header.msg_streamd_id == pre_msg_header.msg_streamd_id
        {
            new_header.basic_header.format = 1;
            new_header.message_header.timestamp -= pre_msg_header.timestamp;

            if cur_msg_header.msg_type_id == pre_msg_header.msg_type_id
                && cur_msg_header.msg_length == pre_msg_header.msg_length
            {
                new_header.basic_header.format = 2;

                if new_header.message_header.timestamp == pre_msg_header.timestamp {
                    new_header.basic_header.format = 3;
                }
            }
        }

        Ok(PackResult::Success)
    }

    fn write_chunk(self, chunk_info: &ChunkInfo) {
        self.zip_chunk_header(chunk_info)
    }
}
