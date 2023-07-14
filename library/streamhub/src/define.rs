use crate::utils;

use {
    super::errors::ChannelError,
    crate::statistics::StreamStatistics,
    crate::stream::StreamIdentifier,
    async_trait::async_trait,
    bytes::BytesMut,
    serde::ser::SerializeStruct,
    serde::Serialize,
    serde::Serializer,
    std::fmt,
    std::sync::Arc,
    tokio::sync::{broadcast, mpsc, oneshot},
    utils::Uuid,
};

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum SubscribeType {
    /* Remote client request playing rtmp stream.*/
    PlayerRtmp,
    /* Remote client request playing http-flv stream.*/
    PlayerHttpFlv,
    /* Remote client request playing hls stream.*/
    PlayerHls,
    /* Remote client request playing rtsp stream.*/
    PlayerRtsp,
    GenerateHls,
    /* Local client *subscribe* from local rtmp session
    and *publish* (relay push) the stream to remote server.*/
    PublisherRtmp,
}

//session publish type
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum PublishType {
    /* Receive rtmp stream from remote push client */
    PushRtmp,
    /* Local client *publish* the rtmp stream to local session,
    the rtmp stream is *subscribed* (pull) from remote server.*/
    RelayRtmp,
    /* Receive rtsp stream from remote push client */
    PushRtsp,
    RelayRtsp,
}

#[derive(Debug, Serialize, Clone)]
pub struct NotifyInfo {
    pub request_url: String,
    pub remote_addr: String,
}

#[derive(Debug, Clone)]
pub struct SubscriberInfo {
    pub id: Uuid,
    pub sub_type: SubscribeType,
    pub notify_info: NotifyInfo,
}

impl Serialize for SubscriberInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SubscriberInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct PublisherInfo {
    pub id: Uuid,
    pub pub_type: PublishType,
    pub notify_info: NotifyInfo,
}

impl Serialize for PublisherInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("PublisherInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.pub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Clone)]
pub enum FrameData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
}

//used to save data which needs to be transferred between client/server sessions
#[derive(Clone)]
pub enum Information {
    Sdp { data: String },
}

pub type FrameDataSender = mpsc::UnboundedSender<FrameData>;
pub type FrameDataReceiver = mpsc::UnboundedReceiver<FrameData>;

pub type InformationSender = mpsc::UnboundedSender<Information>;
pub type InformationReceiver = mpsc::UnboundedReceiver<Information>;

pub type StreamHubEventSender = mpsc::UnboundedSender<StreamHubEvent>;
pub type StreamHubEventReceiver = mpsc::UnboundedReceiver<StreamHubEvent>;

pub type ClientEventProducer = broadcast::Sender<ClientEvent>;
pub type ClientEventConsumer = broadcast::Receiver<ClientEvent>;

pub type TransmitterEventProducer = mpsc::UnboundedSender<TransmitterEvent>;
pub type TransmitterEventConsumer = mpsc::UnboundedReceiver<TransmitterEvent>;

pub type AvStatisticSender = mpsc::UnboundedSender<StreamStatistics>;
pub type AvStatisticReceiver = mpsc::UnboundedReceiver<StreamStatistics>;

pub type StreamStatisticSizeSender = oneshot::Sender<usize>;
pub type StreamStatisticSizeReceiver = oneshot::Sender<usize>;

#[async_trait]
pub trait TStreamHandler: Send + Sync {
    async fn send_cache_data(
        &self,
        sender: FrameDataSender,
        sub_type: SubscribeType,
    ) -> Result<(), ChannelError>;
    async fn get_statistic_data(&self) -> Option<StreamStatistics>;
    async fn send_information(&self, sender: InformationSender);
}

#[derive(Serialize)]
pub enum StreamHubEvent {
    Subscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
        #[serde(skip_serializing)]
        sender: FrameDataSender,
    },
    UnSubscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    Publish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
        #[serde(skip_serializing)]
        receiver: FrameDataReceiver,
        #[serde(skip_serializing)]
        stream_handler: Arc<dyn TStreamHandler>,
    },
    UnPublish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    #[serde(skip_serializing)]
    ApiStatistic {
        data_sender: AvStatisticSender,
        size_sender: StreamStatisticSizeSender,
    },
    #[serde(skip_serializing)]
    ApiKickClient { id: Uuid },

    #[serde(skip_serializing)]
    Request {
        identifier: StreamIdentifier,
        sender: InformationSender,
    },
}

#[derive(Debug)]
pub enum TransmitterEvent {
    Subscribe {
        sender: FrameDataSender,
        info: SubscriberInfo,
    },
    UnSubscribe {
        info: SubscriberInfo,
    },
    UnPublish {},

    Api {
        sender: AvStatisticSender,
    },
    Request {
        sender: InformationSender,
    },
}

impl fmt::Display for TransmitterEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    /*Need publish(push) a stream to other rtmp server*/
    Publish { identifier: StreamIdentifier },
    UnPublish { identifier: StreamIdentifier },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe { identifier: StreamIdentifier },
    UnSubscribe { identifier: StreamIdentifier },
}

//Used for kickoff
#[derive(Debug, Clone)]
pub enum PubSubInfo {
    Subscribe {
        identifier: StreamIdentifier,
        sub_info: SubscriberInfo,
    },

    Publish {
        identifier: StreamIdentifier,
    },
}
