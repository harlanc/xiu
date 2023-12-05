pub mod errors;

use streamhub::{
    define::{
        FrameData, InformationSender, NotifyInfo, PublishType, PublisherInfo, StreamHubEvent,
        StreamHubEventSender, SubscribeType, TStreamHandler,
    },
    errors::ChannelError,
    statistics::StreamStatistics,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use bytesio::bytesio::UdpIO;
use errors::SessionError;
use errors::SessionErrorValue;

use std::fs::File;
use std::{sync::Arc, time::SystemTime};
use streamhub::define::DataReceiver;
use streamhub::define::DataSender;

use tokio::sync::mpsc;

use async_trait::async_trait;
use bytesio::bytesio::TNetIO;

use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use xmpegts::ps::errors::MpegPsError;
use xmpegts::ps::errors::MpegPsErrorValue;
use xmpegts::ps::ps_demuxer::PsDemuxer;
use xmpegts::{define::epsi_stream_type, errors::MpegErrorValue};

use std::io::prelude::*;
use xrtsp::rtp::RtpPacket;
use xrtsp::rtp::{rtp_queue::RtpQueue, utils::Unmarshal};

pub struct GB28181ServerSession {
    pub session_id: Uuid,
    pub local_port: u16,
    stream_name: String,
    io: Box<dyn TNetIO + Send + Sync>,
    event_sender: StreamHubEventSender,
    stream_handler: Arc<GB28181StreamHandler>,
    dump_file: Option<File>,
    dump_last_recv_timestamp: u64,
    pub exit_sender: UnboundedSender<()>,
    exit_receiver: UnboundedReceiver<()>,
}

pub fn print(data: BytesMut) {
    println!("==========={}", data.len());
    let mut idx = 0;
    for i in data {
        print!("{i:02X} ");
        idx += 1;
        if idx % 16 == 0 {
            println!()
        }
    }
    println!("===========")
}

pub fn current_time() -> u64 {
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match duration {
        Ok(result) => (result.as_millis()) as u64,
        _ => 0,
    }
}

impl GB28181ServerSession {
    pub async fn new(
        // stream: UdpIO,
        local_port: u16,
        event_sender: StreamHubEventSender,
        stream_name: String,
        need_dump: bool,
    ) -> Option<Self> {
        let stream_handler = Arc::new(GB28181StreamHandler::new());
        let session_id = Uuid::new(RandomDigitCount::Zero);

        let dump_file = if need_dump {
            let file_handler = File::create(format!("./{stream_name}.dump")).unwrap();
            Some(file_handler)
        } else {
            None
        };

        if let Some(udp_io) = UdpIO::new(local_port, None).await {
            let local_port = udp_io.get_local_port().unwrap();
            let io: Box<dyn TNetIO + Send + Sync> = Box::new(udp_io);

            let (exit_sender, exit_receiver) = mpsc::unbounded_channel::<()>();

            return Some(Self {
                local_port,
                session_id,
                io,
                stream_name,
                event_sender,
                stream_handler,
                dump_file,
                dump_last_recv_timestamp: 0,
                exit_sender,
                exit_receiver,
            });
        }
        None
    }

    pub fn dump(&mut self, data: &BytesMut) {
        if let Some(f) = &mut self.dump_file {
            let cur_time_delta = if self.dump_last_recv_timestamp == 0 {
                self.dump_last_recv_timestamp = current_time();
                0
            } else {
                let cur_time = current_time();
                let cur_time_delta = (cur_time - self.dump_last_recv_timestamp) as u16;
                self.dump_last_recv_timestamp = cur_time;
                cur_time_delta
            };

            if let Err(err) = f.write_all(&cur_time_delta.to_be_bytes()) {
                log::error!("dump time delta err: {}", err);
            }

            let length = data.len() as u16;
            log::trace!("dump length: {}", length);
            if let Err(err) = f.write_all(&length.to_be_bytes()) {
                log::error!("dump length err: {}", err);
            }

            if let Err(err) = f.write_all(&data[..]) {
                log::error!("dump data err: {}", err);
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.publish_to_stream_hub(receiver)?;
        let mut ps_demuxer = self.new_ps_demuxer(sender);

        let mut bytes_reader = BytesReader::new(BytesMut::default());
        let mut rtp_queue = RtpQueue::new(200);

        loop {
            tokio::select! {
                rv = self.io.read() => {
                    let data = rv?;
                    self.dump(&data);
                    bytes_reader.extend_from_slice(&data[..]);

                    let rtp_packet = RtpPacket::unmarshal(&mut bytes_reader)?;
                    rtp_queue.write_queue(rtp_packet);

                    while let Some(rtp_packet) = rtp_queue.read_queue() {
                        if let Err(err) = ps_demuxer.demux(rtp_packet.payload) {
                            match err.value {
                                MpegErrorValue::MpegPsError(ps_err) => match ps_err.value {
                                    MpegPsErrorValue::NotEnoughBytes => {
                                        continue;
                                    }
                                    _ => {
                                        return Err(SessionError {
                                            value: SessionErrorValue::MpegError(ps_err.into()),
                                        });
                                    }
                                },
                                _ => {
                                    return Err(SessionError {
                                        value: SessionErrorValue::MpegError(err),
                                    });
                                }
                            }
                        }
                    }
                }
                _ = self.exit_receiver.recv()=>{
                    self.unpublish_to_stream_hub()?;
                    break;
                }
            }
        }

        Ok(())
    }

    fn new_ps_demuxer(&self, sender: UnboundedSender<FrameData>) -> PsDemuxer {
        let handler = Box::new(
            move |pts: u64,
                  _dts: u64,
                  stream_type: u8,
                  payload: BytesMut|
                  -> Result<(), MpegPsError> {
                match stream_type {
                    epsi_stream_type::PSI_STREAM_H264 | epsi_stream_type::PSI_STREAM_H265 => {
                        let video_frame_data = FrameData::Video {
                            timestamp: pts as u32,
                            data: payload,
                        };
                        log::trace!("receive video data");
                        if let Err(err) = sender.send(video_frame_data) {
                            log::error!("send video frame err: {}", err);
                        }
                    }
                    epsi_stream_type::PSI_STREAM_AAC => {
                        let audio_frame_data = FrameData::Audio {
                            timestamp: pts as u32,
                            data: payload,
                        };
                        log::trace!("receive audio data");
                        if let Err(err) = sender.send(audio_frame_data) {
                            log::error!("send audio frame err: {}", err);
                        }
                    }
                    _ => {}
                }
                Ok(())
            },
        );

        PsDemuxer::new(handler)
    }

    pub fn publish_to_stream_hub(
        &mut self,
        receiver: UnboundedReceiver<FrameData>,
    ) -> Result<(), SessionError> {
        let publisher_info = PublisherInfo {
            id: self.session_id,
            pub_type: PublishType::PushPsStream,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::GB28181 {
                stream_name: self.stream_name.clone(),
            },
            receiver: DataReceiver {
                frame_receiver: Some(receiver),
                packet_receiver: None,
            },
            info: publisher_info,
            stream_handler: self.stream_handler.clone(),
        };
        if self.event_sender.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }
        Ok(())
    }

    pub fn unpublish_to_stream_hub(&self) -> Result<(), SessionError> {
        let unpublish_event = StreamHubEvent::UnPublish {
            identifier: StreamIdentifier::GB28181 {
                stream_name: self.stream_name.clone(),
            },
            info: PublisherInfo {
                id: self.session_id,
                pub_type: PublishType::PushPsStream,
                notify_info: NotifyInfo {
                    request_url: String::from(""),
                    remote_addr: String::from(""),
                },
            },
        };

        let rv = self.event_sender.send(unpublish_event);
        match rv {
            Err(_) => {
                log::error!(
                    "unpublish_to_stream_hub error.stream_name: {}",
                    self.stream_name
                );
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                log::info!(
                    "unpublish_to_stream_hub successfully.stream name: {}",
                    self.stream_name
                );
                Ok(())
            }
        }
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
