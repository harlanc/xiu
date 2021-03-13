use bytes::BytesMut;
pub enum ChannelMsgData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData {},
}
