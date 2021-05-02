use {
    crate::amf0::define::Amf0ValueType,
    bytes::BytesMut,
    std::collections::HashMap,
    tokio::sync::{broadcast, mpsc, oneshot},
};
#[derive(Clone)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { body: BytesMut },
}

pub type ChannelDataProducer = mpsc::UnboundedSender<ChannelData>;
pub type ChannelDataConsumer = mpsc::UnboundedReceiver<ChannelData>;

pub type ClientEventProducer = broadcast::Sender<ClientEvent>;
pub type ClientEventConsumer = broadcast::Receiver<ClientEvent>;

pub type ChannelEventProducer = mpsc::UnboundedSender<ChannelEvent>;
pub type ChannelEventConsumer = mpsc::UnboundedReceiver<ChannelEvent>;

pub type TransmitEventPublisher = mpsc::UnboundedSender<TransmitEvent>;
pub type TransmitEventConsumer = mpsc::UnboundedReceiver<TransmitEvent>;

type ChannelResponder<T> = oneshot::Sender<T>;

pub enum ChannelEvent {
    Subscribe {
        app_name: String,
        stream_name: String,
        session_id: u64,
        responder: ChannelResponder<ChannelDataConsumer>,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
        session_id: u64,
    },
    Publish {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<ChannelDataProducer>,
        connect_command_object: HashMap<String, Amf0ValueType>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
}

pub enum TransmitEvent {
    Subscribe {
        responder: ChannelResponder<ChannelDataConsumer>,
        session_id: u64,
    },

    UnSubscribe {
        session_id: u64,
    },

    UnPublish {},
}
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Publish {
        app_name: String,
        stream_name: String,
        connect_command_object: HashMap<String, Amf0ValueType>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
}
