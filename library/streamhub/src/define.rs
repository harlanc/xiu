use chrono::{DateTime, Local};
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

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum SubscribeType {
    /* Remote client request playing rtmp stream.*/
    PlayerRtmp,
    /* Remote client request playing http-flv stream.*/
    PlayerHttpFlv,
    /* Remote client request playing hls stream.*/
    PlayerHls,
    /* Remote/local client request playing rtsp stream.*/
    PlayerRtsp,
    /* Local client request playing webrtc stream, it's used for protocol remux.*/
    PlayerWebrtc,
    /* Remote client request playing rtsp or webrtc(whep) raw rtp stream.*/
    PlayerRtp,
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
    /* Receive webrtc stream from remote push client(whip),  */
    PushWebRTC,
    /* It used for publishing raw rtp data of rtsp/whbrtc(whip) */
    PushRtp,
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
    Request {
        identifier: StreamIdentifier,
        sender: InformationSender,
    },
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
    Publish { identifier: StreamIdentifier },
    UnPublish { identifier: StreamIdentifier },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe { identifier: StreamIdentifier },
    UnSubscribe { identifier: StreamIdentifier },
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
