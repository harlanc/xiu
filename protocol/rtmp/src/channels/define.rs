use {
    bytes::BytesMut,
    tokio::sync::{broadcast, mpsc, oneshot},
};
#[derive(Clone)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { body: BytesMut },
}

pub type ChannelDataPublisher = mpsc::UnboundedSender<ChannelData>;
pub type ChannelDataConsumer = mpsc::UnboundedReceiver<ChannelData>;

pub type ChanPublisher = broadcast::Sender<ChannelDataPublisher>;
pub type ChanConsumer = broadcast::Receiver<ChannelDataConsumer>;

pub type ChannelEventPublisher = mpsc::UnboundedSender<ChannelEvent>;
pub type ChannelEventConsumer = mpsc::UnboundedReceiver<ChannelEvent>;

pub type TransmitEventPublisher = mpsc::UnboundedSender<TransmitEvent>;
pub type TransmitEventConsumer = mpsc::UnboundedReceiver<TransmitEvent>;

type ChannelResponder<T> = oneshot::Sender<T>;

pub enum ChannelEvent {
    Subscribe {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<ChannelDataConsumer>,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
    },
    Publish {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<ChannelDataPublisher>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
}

pub enum TransmitEvent {
    Subscribe {
        responder: ChannelResponder<ChannelDataConsumer>,
    },
}
