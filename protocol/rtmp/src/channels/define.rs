use bytes::BytesMut;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Clone)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData {},
}

pub type SingleProducerForData = broadcast::Sender<ChannelData>;
pub type MultiConsumerForData = broadcast::Receiver<ChannelData>;

pub type MultiProducerForEvent = mpsc::UnboundedSender<ChannelEvent>;
pub type SingleConsumerForEvent = mpsc::UnboundedReceiver<ChannelEvent>;

type ChannelResponder<T> = oneshot::Sender<T>;

pub enum ChannelEvent {
    Subscribe {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<MultiConsumerForData>,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
    },
    Publish {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<SingleProducerForData>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
}
