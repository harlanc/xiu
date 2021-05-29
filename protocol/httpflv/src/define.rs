use {
    bytes::BytesMut,
    tokio::sync::{broadcast, mpsc, oneshot},
};

pub mod tag_type {
    pub const audio: u8 = 8;
    pub const video: u8 = 9;
    pub const script_data_amf: u8 = 18;
}
pub type HttpResponseDataProducer = mpsc::UnboundedSender<BytesMut>;
pub type HttpResponseDataConsumer = mpsc::UnboundedReceiver<BytesMut>;
