use {
    bytes::BytesMut,
    tokio::sync::{broadcast, mpsc, oneshot},
    std::io,
};

use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::mpsc::UnboundedSender;

pub mod tag_type {
    pub const audio: u8 = 8;
    pub const video: u8 = 9;
    pub const script_data_amf: u8 = 18;
}
pub type HttpResponseDataProducer = UnboundedSender<io::Result<BytesMut>>;
pub type HttpResponseDataConsumer = UnboundedReceiver<io::Result<BytesMut>>;
