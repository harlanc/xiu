use {
    crate::session::common::SessionInfo,
    bytes::BytesMut,
    std::fmt,
    tokio::sync::{broadcast, mpsc, oneshot},
};
#[derive(Clone)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
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
#[derive(Debug)]
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

impl fmt::Display for ChannelEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelEvent::Subscribe {
                app_name,
                stream_name,
                session_info,
                responder: _,
            } => {
                write!(
                    f,
                    "receive event, event_name: Subscribe, app_name: {},stream_name: {}, subscriber id: {}",
                    app_name, stream_name, session_info.subscriber_id,
                )
            }
            ChannelEvent::UnSubscribe {
                app_name,
                stream_name,
                session_info,
            } => {
                write!(
                    f,
                    "receive event, event_name: UnSubscribe, app_name: {},stream_name: {}, subscriber id: {}",
                    app_name, stream_name, session_info.subscriber_id,
                )
            }
            ChannelEvent::Publish {
                app_name,
                stream_name,
                responder: _,
            } => {
                write!(
                    f,
                    "receive event, event_name: Publish, app_name: {},stream_name: {}",
                    app_name, stream_name,
                )
            }
            ChannelEvent::UnPublish {
                app_name,
                stream_name,
            } => {
                write!(
                    f,
                    "receive event, event_name: UnPublish, app_name: {},stream_name: {}",
                    app_name, stream_name,
                )
            }
        }
    }
}

#[derive(Debug)]
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

impl fmt::Display for TransmitEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    /*Need publish(push) a stream to other rtmp server*/
    Publish {
        app_name: String,
        stream_name: String,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe {
        app_name: String,
        stream_name: String,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
    },
}
