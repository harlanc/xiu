use {
    futures::channel::mpsc::{UnboundedReceiver, UnboundedSender},
    {bytes::BytesMut, std::io},
};
pub mod tag_type {
    pub const AUDIO: u8 = 8;
    pub const VIDEO: u8 = 9;
    pub const SCRIPT_DATA_AMF: u8 = 18;
}
pub type HttpResponseDataProducer = UnboundedSender<io::Result<BytesMut>>;
pub type HttpResponseDataConsumer = UnboundedReceiver<io::Result<BytesMut>>;
