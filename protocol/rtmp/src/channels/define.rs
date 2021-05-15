use {
    crate::session::common::SessionInfo,
    bytes::BytesMut,
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
        session_info: SessionInfo,
        responder: ChannelResponder<ChannelDataConsumer>,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
        session_info: SessionInfo,
    },
    Publish {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<ChannelDataProducer>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
}

pub enum TransmitEvent {
    Subscribe {
        responder: ChannelResponder<ChannelDataConsumer>,
        session_info: SessionInfo,
    },

    UnSubscribe {
        session_info: SessionInfo,
    },

    UnPublish {},
}
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Publish {
        app_name: String,
        stream_name: String,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
    Subscribe {
        app_name: String,
        stream_name: String,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
    },
}
