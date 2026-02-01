use chrono::{DateTime, Local};
use serde::Deserialize;
use serde_json::Value;
use xflv::define::{AacProfile, AvcCodecId, AvcLevel, AvcProfile, SoundFormat};

use crate::utils;

use {
    super::errors::StreamHubError,
    crate::statistics::StatisticsStream,
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

/* Subscribe streams from stream hub */
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum SubscribeType {
    /* Remote client request pulling(play) a rtmp stream.*/
    RtmpPull,
    /* Remote request to play httpflv triggers remux from RTMP to httpflv. */
    RtmpRemux2HttpFlv,
    /* The publishing of RTMP stream triggers remuxing from RTMP to HLS protocol.(NOTICE:It is not triggerred by players.)*/
    RtmpRemux2Hls,
    /* Relay(Push) local RTMP stream from stream hub to other RTMP nodes.*/
    RtmpRelay,
    /* Remote client request pulling(play) a rtsp stream.*/
    RtspPull,
    /* The publishing of RTSP stream triggers remuxing from RTSP to RTMP protocol.*/
    RtspRemux2Rtmp,
    /* Relay(Push) local RTSP stream to other RTSP nodes.*/
    RtspRelay,
    /* Remote client request pulling(play) stream through whep.*/
    WhepPull,
    /* Remuxing webrtc stream to RTMP */
    WebRTCRemux2Rtmp,
    /* Relay(Push) the local webRTC stream to other nodes using Whip.*/
    WhipRelay,
    /* Pull rtp stream by subscribing from stream hub.*/
    RtpPull,
}

/* Publish streams to stream hub */
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum PublishType {
    /* Receive rtmp stream from remote push client. */
    RtmpPush,
    /* Relay(Pull) remote RTMP stream to local stream hub. */
    RtmpRelay,
    /* Receive rtsp stream from remote push client */
    RtspPush,
    /* Relay(Pull) remote RTSP stream to local stream hub. */
    RtspRelay,
    /* Receive whip stream from remote push client. */
    WhipPush,
    /* Relay(Pull) remote WebRTC stream to local stream hub using Whep. */
    WhepRelay,
    /* It used for publishing raw rtp data of rtsp/whbrtc(whip) */
    RtpPush,
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
    pub sub_data_type: SubDataType,
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
    pub pub_data_type: PubDataType,
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
        state.serialize_field("pub_type", &self.pub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Clone, PartialEq)]
pub enum VideoCodecType {
    H264,
    H265,
}

#[derive(Clone)]
pub struct MediaInfo {
    pub audio_clock_rate: u32,
    pub video_clock_rate: u32,
    pub vcodec: VideoCodecType,
}

#[derive(Clone)]
pub enum FrameData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
    MediaInfo { media_info: MediaInfo },
}

//Used to pass rtp raw data.
#[derive(Clone)]
pub enum PacketData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
}

//used to save data which needs to be transferred between client/server sessions
#[derive(Clone)]
pub enum Information {
    Sdp { data: String },
}

//used to transfer a/v frame between different protocols(rtmp/rtsp/webrtc/http-flv/hls)
//or send a/v frame data from publisher to subscribers.
pub type FrameDataSender = mpsc::UnboundedSender<FrameData>;
pub type FrameDataReceiver = mpsc::UnboundedReceiver<FrameData>;

//used to transfer rtp packet data,it includles the following directions:
// rtsp(publisher)->stream hub->rtsp(subscriber)
// webrtc(publisher whip)->stream hub->webrtc(subscriber whep)
pub type PacketDataSender = mpsc::UnboundedSender<PacketData>;
pub type PacketDataReceiver = mpsc::UnboundedReceiver<PacketData>;

pub type InformationSender = mpsc::UnboundedSender<Information>;
pub type InformationReceiver = mpsc::UnboundedReceiver<Information>;

pub type StreamHubEventSender = mpsc::UnboundedSender<StreamHubEvent>;
pub type StreamHubEventReceiver = mpsc::UnboundedReceiver<StreamHubEvent>;

pub type BroadcastEventSender = broadcast::Sender<BroadcastEvent>;
pub type BroadcastEventReceiver = broadcast::Receiver<BroadcastEvent>;

pub type TransceiverEventSender = mpsc::UnboundedSender<TransceiverEvent>;
pub type TransceiverEventReceiver = mpsc::UnboundedReceiver<TransceiverEvent>;

pub type StatisticDataSender = mpsc::UnboundedSender<StatisticData>;
pub type StatisticDataReceiver = mpsc::UnboundedReceiver<StatisticData>;

pub type StatisticStreamSender = mpsc::UnboundedSender<StatisticsStream>;
pub type StatisticStreamReceiver = mpsc::UnboundedReceiver<StatisticsStream>;

pub type StatisticApiResultSender = oneshot::Sender<Value>;
pub type StatisticApiResultReceiver = oneshot::Receiver<Value>;

pub type SubEventExecuteResultSender =
    oneshot::Sender<Result<(DataReceiver, Option<StatisticDataSender>), StreamHubError>>;
pub type PubEventExecuteResultSender = oneshot::Sender<
    Result<
        (
            Option<FrameDataSender>,
            Option<PacketDataSender>,
            Option<StatisticDataSender>,
        ),
        StreamHubError,
    >,
>;
// The trait bound `BroadcastEvent: Clone` should be satisfied, so here we cannot use oneshot.
pub type BroadcastEventExecuteResultSender = mpsc::Sender<Result<(), StreamHubError>>;
pub type ApiRelayStreamResultSender = oneshot::Sender<Result<(), StreamHubError>>;
pub type TransceiverEventExecuteResultSender = oneshot::Sender<StatisticDataSender>;

#[async_trait]
pub trait TStreamHandler: Send + Sync {
    async fn send_prior_data(
        &self,
        sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError>;
    async fn get_statistic_data(&self) -> Option<StatisticsStream>;
    async fn send_information(&self, sender: InformationSender);
}

//A publisher can publish one or two kinds of av stream at a time.
pub struct DataReceiver {
    pub frame_receiver: Option<FrameDataReceiver>,
    pub packet_receiver: Option<PacketDataReceiver>,
}

//A subscriber only needs to subscribe to one type of stream at a time
#[derive(Debug, Clone)]
pub enum DataSender {
    Frame { sender: FrameDataSender },
    Packet { sender: PacketDataSender },
}
//we can only sub one kind of stream.
#[derive(Debug, Clone, Serialize)]
pub enum SubDataType {
    Frame,
    Packet,
}
//we can pub frame or packet or both.
#[derive(Debug, Clone, Serialize)]
pub enum PubDataType {
    Frame,
    Packet,
    Both,
}

#[derive(Clone, Serialize, Debug)]
pub enum StreamHubEventMessage {
    Subscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    UnSubscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    Publish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    UnPublish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    OnHls {
        identifier: StreamIdentifier,
        segment: Segment,
    },
    NotSupport {},
}

//we can pub frame or packet or both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayType {
    Pull,
    Push,
}

#[derive(Serialize)]
pub enum StreamHubEvent {
    Subscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
        #[serde(skip_serializing)]
        result_sender: SubEventExecuteResultSender,
    },
    UnSubscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    Publish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
        #[serde(skip_serializing)]
        result_sender: PubEventExecuteResultSender,
        #[serde(skip_serializing)]
        stream_handler: Arc<dyn TStreamHandler>,
    },
    UnPublish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    #[serde(skip_serializing)]
    ApiStatistic {
        top_n: Option<usize>,
        identifier: Option<StreamIdentifier>,
        uuid: Option<Uuid>,
        result_sender: StatisticApiResultSender,
    },
    #[serde(skip_serializing)]
    ApiKickClient { id: Uuid },
    #[serde(skip_serializing)]
    ApiStartRelayStream {
        id: String,
        identifier: StreamIdentifier,
        server_address: String,
        relay_type: RelayType,
        result_sender: ApiRelayStreamResultSender,
    },
    #[serde(skip_serializing)]
    ApiStopRelayStream {
        id: String,
        relay_type: RelayType,
        result_sender: ApiRelayStreamResultSender,
    },
    #[serde(skip_serializing)]
    Request {
        identifier: StreamIdentifier,
        sender: InformationSender,
    },
    OnHls {
        identifier: StreamIdentifier,
        segment: Segment,
    }
}

impl StreamHubEvent {
    pub fn to_message(&self) -> StreamHubEventMessage {
        match self {
            StreamHubEvent::Subscribe {
                identifier,
                info,
                result_sender: _result_sender,
            } => StreamHubEventMessage::Subscribe {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            StreamHubEvent::UnSubscribe { identifier, info } => {
                StreamHubEventMessage::UnSubscribe {
                    identifier: identifier.clone(),
                    info: info.clone(),
                }
            }
            StreamHubEvent::Publish {
                identifier,
                info,
                result_sender: _result_sender,
                stream_handler: _stream_handler,
            } => StreamHubEventMessage::Publish {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            StreamHubEvent::UnPublish { identifier, info } => StreamHubEventMessage::UnPublish {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            StreamHubEvent::OnHls { identifier, segment } => StreamHubEventMessage::OnHls {
                identifier: identifier.clone(),
                segment: segment.clone(),
            },
            _ => StreamHubEventMessage::NotSupport {},
        }
    }
}

#[derive(Debug)]
pub enum TransceiverEvent {
    Subscribe {
        sender: DataSender,
        info: SubscriberInfo,
        result_sender: TransceiverEventExecuteResultSender,
    },
    UnSubscribe {
        info: SubscriberInfo,
    },
    UnPublish {},

    Api {
        sender: StatisticStreamSender,
        uuid: Option<Uuid>,
    },
    Request {
        sender: InformationSender,
    },
}

impl fmt::Display for TransceiverEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone)]
pub enum BroadcastEvent {
    /*Need publish(push) a stream to other rtmp server*/
    Publish {
        identifier: StreamIdentifier,
    },
    UnPublish {
        identifier: StreamIdentifier,
    },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe {
        id: String,
        identifier: StreamIdentifier,
        server_address: Option<String>,
        result_sender: Option<BroadcastEventExecuteResultSender>,
    },
    UnSubscribe {
        id: String,
        result_sender: Option<BroadcastEventExecuteResultSender>,
        //identifier: StreamIdentifier,
        //server_address: Option<String>,
    },
}

pub enum StatisticData {
    AudioCodec {
        sound_format: SoundFormat,
        profile: AacProfile,
        samplerate: u32,
        channels: u8,
    },
    VideoCodec {
        codec: AvcCodecId,
        profile: AvcProfile,
        level: AvcLevel,
        width: u32,
        height: u32,
    },
    Audio {
        uuid: Option<Uuid>,
        data_size: usize,
        aac_packet_type: u8,
        duration: usize,
    },
    Video {
        uuid: Option<Uuid>,
        data_size: usize,
        frame_count: usize,
        is_key_frame: Option<bool>,
        duration: usize,
    },
    Publisher {
        id: Uuid,
        remote_addr: String,
        start_time: DateTime<Local>,
    },
    Subscriber {
        id: Uuid,
        remote_addr: String,
        sub_type: SubscribeType,
        start_time: DateTime<Local>,
    },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /*ts duration*/
    pub duration: i64,
    pub discontinuity: bool,
    /*ts name*/
    pub name: String,
    pub path: String,
    pub is_eof: bool,
}

impl Segment {
    pub fn new(
        duration: i64,
        discontinuity: bool,
        name: String,
        path: String,
        is_eof: bool,
    ) -> Self {
        Self {
            duration,
            discontinuity,
            name,
            path,
            is_eof,
        }
    }
}
