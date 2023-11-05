pub mod errors;

use streamhub::{
    define::{
        FrameData, Information, InformationSender, NotifyInfo, PublishType, PublisherInfo,
        StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::{ChannelError, ChannelErrorValue},
    statistics::StreamStatistics,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use bytesio::bytesio::UdpIO;
use errors::SessionError;
use errors::SessionErrorValue;
use http::StatusCode;
use std::collections::HashMap;
use std::sync::Arc;
use streamhub::define::DataReceiver;
use streamhub::define::DataSender;
use streamhub::define::MediaInfo;
use streamhub::define::VideoCodecType;
use tokio::sync::mpsc;

use async_trait::async_trait;
use bytesio::bytesio::TNetIO;
use bytesio::bytesio::TcpIO;

use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;
use xmpegts::define::epsi_stream_type;
use xmpegts::ps::errors::MpegPsError;
use xmpegts::ps::ps_demuxer::PsDemuxer;

use xrtsp::rtp::RtpPacket;
use xrtsp::rtp::{rtp_queue::RtpQueue, utils::Unmarshal};

pub struct GB28181ServerSession {
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,
    ps_demuxer: PsDemuxer,
    rtp_queue: RtpQueue,
    event_sender: StreamHubEventSender,
    stream_handler: Arc<GB28181StreamHandler>,
}

impl GB28181ServerSession {
    pub fn new(
        stream: TcpStream,
        event_sender: StreamHubEventSender,
    ) -> Result<Self, SessionError> {
        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        let publisher_info = PublisherInfo {
            id: Uuid::new(RandomDigitCount::Zero),
            pub_type: PublishType::PushPsStream,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let stream_handler = Arc::new(GB28181StreamHandler::new());

        let (sender, receiver) = mpsc::unbounded_channel();
        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::GB28181 {
                stream_name: String::from("test"),
            },
            receiver: DataReceiver {
                frame_receiver: Some(receiver),
                packet_receiver: None,
            },
            info: publisher_info,
            stream_handler: stream_handler.clone(),
        };

        if event_sender.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let handler = Box::new(
            move |pts: u64,
                  dts: u64,
                  stream_type: u8,
                  payload: BytesMut|
                  -> Result<(), MpegPsError> {
                match stream_type {
                    epsi_stream_type::PSI_STREAM_H264 | epsi_stream_type::PSI_STREAM_H265 => {
                        let video_frame_data = FrameData::Video {
                            timestamp: dts as u32,
                            data: payload,
                        };

                        if let Err(err) = sender.send(video_frame_data) {
                            log::error!("send video frame err: {}", err);
                        }
                    }
                    epsi_stream_type::PSI_STREAM_AAC => {
                        let audio_frame_data = FrameData::Audio {
                            timestamp: dts as u32,
                            data: payload,
                        };

                        if let Err(err) = sender.send(audio_frame_data) {
                            log::error!("send audio frame err: {}", err);
                        }
                    }
                    _ => {}
                }

                Ok(())
            },
        );

        Ok(Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            ps_demuxer: PsDemuxer::new(handler),
            event_sender,
            stream_handler,
            rtp_queue: RtpQueue::new(200),
        })
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            let data = self.io.lock().await.read().await?;
            self.reader.extend_from_slice(&data[..]);
            let rtp_packet = RtpPacket::unmarshal(&mut self.reader)?;

            self.rtp_queue.write_queue(rtp_packet);

            loop {
                if let Some(rtp_packet) = self.rtp_queue.read_queue() {
                    self.ps_demuxer.demux(rtp_packet.payload)?;
                } else {
                    break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct GB28181StreamHandler {}

impl GB28181StreamHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TStreamHandler for GB28181StreamHandler {
    async fn send_prior_data(
        &self,
        _sender: DataSender,
        _sub_type: SubscribeType,
    ) -> Result<(), ChannelError> {
        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StreamStatistics> {
        None
    }

    async fn send_information(&self, _sender: InformationSender) {}
}
