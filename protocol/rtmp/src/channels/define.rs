use bytes::BytesMut;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Clone,Copy)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData {},
}

// impl Copy for ChannelData{
//     fn Copy(&self) -> ChannelData {
//         match self {
//             &ChannelData::Video{timestamp,data} => &ChannelData::Video{timestamp,data.clone()}
           
//         }
//     }
// }

pub type ChannelDataPublisher = broadcast::Sender<ChannelData>;
pub type ChannelDataConsumer = broadcast::Receiver<ChannelData>;

pub type PlayerPublisher = oneshot::Sender<ChannelData>;
pub type PlayerConsumer = oneshot::Receiver<ChannelData>;

pub type ChannelEventPublisher = mpsc::UnboundedSender<ChannelEvent>;
pub type ChannelEventConsumer = mpsc::UnboundedReceiver<ChannelEvent>;

type ChannelResponder<T> = oneshot::Sender<T>;

pub enum ChannelEvent {
    Subscribe {
        app_name: String,
        stream_name: String,
        responder: ChannelResponder<oneshot::Receiver<ChannelData>>,
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
