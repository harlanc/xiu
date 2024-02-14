pub mod define;
pub mod errors;
use super::rtsp_codec;
use crate::global_trait::Marshal;
use crate::global_trait::Unmarshal;

use crate::rtp::define::ANNEXB_NALU_START_CODE;
use crate::rtp::utils::Marshal as RtpMarshal;
use commonlib::http::HttpRequest as RtspRequest;
use commonlib::http::HttpResponse as RtspResponse;
use commonlib::http::Marshal as RtspMarshal;
use commonlib::http::Unmarshal as RtspUnmarshal;

use crate::rtp::RtpPacket;
use crate::rtsp_range::RtspRange;

use crate::sdp::fmtp::Fmtp;

use crate::rtsp_codec::RtspCodecInfo;
use crate::rtsp_track::RtspTrack;
use crate::rtsp_track::TrackType;
use crate::rtsp_transport::ProtocolType;
use crate::rtsp_transport::RtspTransport;

use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;

use bytesio::bytes_writer::BytesWriter;
use bytesio::bytesio::UdpIO;
use errors::SessionError;
use errors::SessionErrorValue;
use http::StatusCode;
use streamhub::define::DataSender;
use streamhub::define::MediaInfo;
use streamhub::define::VideoCodecType;
use tokio::sync::oneshot;

use super::rtp::errors::UnPackerError;
use super::sdp::Sdp;

use async_trait::async_trait;
use bytesio::bytesio::TNetIO;
use bytesio::bytesio::TcpIO;
use define::rtsp_method_name;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use commonlib::auth::Auth;
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

pub struct RtspServerSession {
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,

    stream_handler: Arc<RtspStreamHandler>,
    event_producer: StreamHubEventSender,

    auth: Option<Auth>,
}

pub struct InterleavedBinaryData {
    channel_identifier: u8,
    length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        log::debug!("dollar sign: {}", is_dollar_sign);
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            log::debug!("channel_identifier: {}", channel_identifier);
            let length = reader.read_u16::<BigEndian>()?;
            log::debug!("length: {}", length);
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
    ) -> Self {
        // let remote_addr = if let Ok(addr) = stream.peer_addr() {
        //     log::info!("server session: {}", addr.to_string());
        //     Some(addr)
        // } else {
        //     None
        // };

        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
            auth,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            while self.reader.len() < 4 {
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
            }

            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
                match data {
                    Some(a) => {
                        if self.reader.len() < a.length as usize {
                            let data = self.io.lock().await.read().await?;
                            self.reader.extend_from_slice(&data[..]);
                        }
                        self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                            .await?;
                    }
                    None => {
                        self.on_rtsp_message().await?;
                    }
                }
            }
        }
    }

    async fn on_rtp_over_rtsp_message(
        &mut self,
        channel_identifier: u8,
        length: usize,
    ) -> Result<(), SessionError> {
        let mut cur_reader = BytesReader::new(self.reader.read_bytes(length)?);

        for track in self.tracks.values_mut() {
            if let Some(interleaveds) = track.transport.interleaved {
                let rtp_identifier = interleaveds[0];
                let rtcp_identifier = interleaveds[1];

                if channel_identifier == rtp_identifier {
                    track.on_rtp(&mut cur_reader).await?;
                } else if channel_identifier == rtcp_identifier {
                    track.on_rtcp(&mut cur_reader, self.io.clone()).await;
                }
            }
        }
        Ok(())
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let data = self.reader.extract_remaining_bytes();

        if let Some(rtsp_request) = RtspRequest::unmarshal(std::str::from_utf8(&data)?) {
            match rtsp_request.method.as_str() {
                rtsp_method_name::OPTIONS => {
                    self.handle_options(&rtsp_request).await?;
                }
                rtsp_method_name::DESCRIBE => {
                    self.handle_describe(&rtsp_request).await?;
                }
                rtsp_method_name::ANNOUNCE => {
                    self.handle_announce(&rtsp_request).await?;
                }
                rtsp_method_name::SETUP => {
                    self.handle_setup(&rtsp_request).await?;
                }
                rtsp_method_name::PLAY => {
                    if self.handle_play(&rtsp_request).await.is_err() {
                        self.unsubscribe_from_stream_hub(rtsp_request.uri.path)?;
                    }
                }
                rtsp_method_name::RECORD => {
                    self.handle_record(&rtsp_request).await?;
                }
                rtsp_method_name::TEARDOWN => {
                    self.handle_teardown(&rtsp_request)?;
                }
                rtsp_method_name::PAUSE => {}
                rtsp_method_name::GET_PARAMETER => {}
                rtsp_method_name::SET_PARAMETER => {}
                rtsp_method_name::REDIRECT => {}

                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_options(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        let public_str = rtsp_method_name::ARRAY.join(",");
        response.headers.insert("Public".to_string(), public_str);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;

        // The sender is used for sending sdp information from the server session to client session
        // receiver is used to receive the sdp information
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let request_event = StreamHubEvent::Request {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.uri.path.clone(),
            },
            sender,
        };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await {
            if let Some(sdp) = Sdp::unmarshal(&data) {
                self.sdp = sdp;
                //it can new tracks when get the sdp information;
                self.new_tracks()?;
            }
        }

        let mut response = Self::gen_response(status_code, rtsp_request);
        let sdp = self.sdp.marshal();
        log::debug!("sdp: {}", sdp);
        response.body = Some(sdp);
        response
            .headers
            .insert("Content-Type".to_string(), "application/sdp".to_string());
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_announce(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(&stream_name, &rtsp_request.uri.query, false)?;
        }

        if let Some(request_body) = &rtsp_request.body {
            if let Some(sdp) = Sdp::unmarshal(request_body) {
                self.sdp = sdp.clone();
                self.stream_handler.set_sdp(sdp).await;
            }
        }

        //new tracks for publish session
        self.new_tracks()?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.uri.path.clone(),
            },
            result_sender: event_result_sender,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let sender = event_result_receiver.await??.0.unwrap();

        for track in self.tracks.values_mut() {
            let sender_out = sender.clone();
            let mut rtp_channel_guard = track.rtp_channel.lock().await;

            rtp_channel_guard.on_frame_handler(Box::new(
                move |msg: FrameData| -> Result<(), UnPackerError> {
                    if let Err(err) = sender_out.send(msg) {
                        log::error!("send frame error: {}", err);
                    }
                    Ok(())
                },
            ));

            let rtcp_channel = Arc::clone(&track.rtcp_channel);
            rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(move |packet: RtpPacket| {
                let rtcp_channel_in = Arc::clone(&rtcp_channel);
                Box::pin(async move {
                    rtcp_channel_in.lock().await.on_packet(packet);
                })
            }));
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, rtsp_request);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        for track in self.tracks.values_mut() {
            if !rtsp_request.uri.marshal().contains(&track.media_control) {
                continue;
            }

            if let Some(transport_data) = rtsp_request.get_header(&"Transport".to_string()) {
                if self.session_id.is_none() {
                    self.session_id = Some(Uuid::new(RandomDigitCount::Zero));
                }

                let transport = RtspTransport::unmarshal(transport_data);

                if let Some(mut trans) = transport {
                    let mut rtp_server_port: Option<u16> = None;
                    let mut rtcp_server_port: Option<u16> = None;

                    match trans.protocol_type {
                        ProtocolType::TCP => {
                            track.create_packer(self.io.clone()).await;
                        }
                        ProtocolType::UDP => {
                            let (rtp_port, rtcp_port) =
                                if let Some(client_ports) = trans.client_port {
                                    (client_ports[0], client_ports[1])
                                } else {
                                    log::error!("should not be here!!");
                                    (0, 0)
                                };

                            let address = rtsp_request.uri.host.clone();
                            if let Some(rtp_io) = UdpIO::new(address.clone(), rtp_port, 0).await {
                                rtp_server_port = rtp_io.get_local_port();

                                let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(rtp_io);
                                //if mode is empty then it is a player session.
                                if trans.transport_mod.is_none() {
                                    track.create_packer(Arc::new(Mutex::new(box_udp_io))).await;
                                } else {
                                    track.rtp_receive_loop(box_udp_io).await;
                                }
                            }

                            if let Some(rtcp_io) =
                                UdpIO::new(address.clone(), rtcp_port, rtp_server_port.unwrap() + 1)
                                    .await
                            {
                                rtcp_server_port = rtcp_io.get_local_port();
                                let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                    Arc::new(Mutex::new(Box::new(rtcp_io)));
                                track.rtcp_receive_loop(box_rtcp_io).await;
                            }
                        }
                    }

                    //tell client the udp ports of server side
                    let mut server_ports: [u16; 2] = [0, 0];
                    if let Some(rtp_port) = rtp_server_port {
                        server_ports[0] = rtp_port;
                    }
                    if let Some(rtcp_server_port) = rtcp_server_port {
                        server_ports[1] = rtcp_server_port;
                        trans.server_port = Some(server_ports);
                    }

                    let new_transport_data = trans.marshal();
                    response
                        .headers
                        .insert("Transport".to_string(), new_transport_data);
                    response
                        .headers
                        .insert("Session".to_string(), self.session_id.unwrap().to_string());

                    track.set_transport(trans).await;
                }
            }
            break;
        }

        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(&stream_name, &rtsp_request.uri.query, true)?;
        }

        for track in self.tracks.values_mut() {
            let protocol_type = track.transport.protocol_type.clone();

            match protocol_type {
                ProtocolType::TCP => {
                    let channel_identifer = if let Some(interleaveds) = track.transport.interleaved
                    {
                        interleaveds[0]
                    } else {
                        log::error!("handle_play:should not be here!!!");
                        0
                    };

                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            Box::pin(async move {
                                let msg = packet.marshal()?;
                                let mut bytes_writer = AsyncBytesWriter::new(io);
                                bytes_writer.write_u8(0x24)?;
                                bytes_writer.write_u8(channel_identifer)?;
                                bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
                ProtocolType::UDP => {
                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            Box::pin(async move {
                                let mut bytes_writer = AsyncBytesWriter::new(io);

                                let msg = packet.marshal()?;
                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
            }
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, rtsp_request);

        self.send_response(&response).await?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let publish_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.uri.path.clone(),
            },
            info: self.get_subscriber_info(),
            result_sender: event_result_sender,
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let mut receiver = event_result_receiver.await??.frame_receiver.unwrap();

        let mut retry_times = 0;
        loop {
            if let Some(frame_data) = receiver.recv().await {
                match frame_data {
                    FrameData::Audio {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(audio_track) = self.tracks.get_mut(&TrackType::Audio) {
                            audio_track
                                .rtp_channel
                                .lock()
                                .await
                                .on_frame(&mut data, timestamp)
                                .await?;
                        }
                    }
                    FrameData::Video {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(video_track) = self.tracks.get_mut(&TrackType::Video) {
                            video_track
                                .rtp_channel
                                .lock()
                                .await
                                .on_frame(&mut data, timestamp)
                                .await?;
                        }
                    }
                    _ => {}
                }
            } else {
                retry_times += 1;
                log::info!(
                    "send_channel_data: no data receives ,retry {} times!",
                    retry_times
                );

                if retry_times > 10 {
                    return Err(SessionError {
                        value: SessionErrorValue::CannotReceiveFrameData,
                    });
                }
            }
        }
    }

    pub fn unsubscribe_from_stream_hub(&mut self, stream_path: String) -> Result<(), SessionError> {
        let identifier = StreamIdentifier::Rtsp { stream_path };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: self.get_subscriber_info(),
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_stream_hub err {}", err);
        }

        Ok(())
    }

    async fn handle_record(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(range_str) = rtsp_request.headers.get(&String::from("Range")) {
            if let Some(range) = RtspRange::unmarshal(range_str) {
                let status_code = http::StatusCode::OK;
                let mut response = Self::gen_response(status_code, rtsp_request);
                response
                    .headers
                    .insert(String::from("Range"), range.marshal());
                response
                    .headers
                    .insert("Session".to_string(), self.session_id.unwrap().to_string());

                self.send_response(&response).await?;
            }
        }

        Ok(())
    }

    fn handle_teardown(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let stream_path = &rtsp_request.uri.path;
        let unpublish_event = StreamHubEvent::UnPublish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: stream_path.clone(),
            },
            info: self.get_publisher_info(),
        };

        let rv = self.event_producer.send(unpublish_event);
        match rv {
            Err(_) => {
                log::error!("unpublish_to_channels error.stream_name: {}", stream_path);
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                log::info!(
                    "unpublish_to_channels successfully.stream name: {}",
                    stream_path
                );
                Ok(())
            }
        }
    }

    fn new_tracks(&mut self) -> Result<(), SessionError> {
        for media in &self.sdp.medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                String::from("")
            };

            let media_name = &media.media_type;
            log::info!("media_name: {}", media_name);
            match media_name.as_str() {
                "audio" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count: media.rtpmap.encoding_param.parse().unwrap(),
                    };

                    log::info!("audio codec info: {:?}", codec_info);

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                }
                "video" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn gen_response(status_code: StatusCode, rtsp_request: &RtspRequest) -> RtspResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            "".to_string()
        };

        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.headers.get("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        response
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::PlayerRtsp,
            sub_data_type: streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        PublisherInfo {
            id,
            pub_type: PublishType::PushRtsp,
            pub_data_type: streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    async fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }
}

#[derive(Default)]
pub struct RtspStreamHandler {
    sdp: Mutex<Sdp>,
}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {
            sdp: Mutex::new(Sdp::default()),
        }
    }
    pub async fn set_sdp(&self, sdp: Sdp) {
        *self.sdp.lock().await = sdp;
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), ChannelError> {
        let sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(ChannelError {
                    value: ChannelErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        match sub_type {
            SubscribeType::PlayerRtmp => {
                let sdp_info = self.sdp.lock().await;
                let mut video_clock_rate: u32 = 0;
                let mut audio_clock_rate: u32 = 0;

                let mut vcodec: VideoCodecType = VideoCodecType::H264;

                for media in &sdp_info.medias {
                    let mut bytes_writer = BytesWriter::new();
                    if let Some(fmtp) = &media.fmtp {
                        match fmtp {
                            Fmtp::H264(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps error: {}", err);
                                }
                                video_clock_rate = media.rtpmap.clock_rate;
                            }
                            Fmtp::H265(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.vps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps/vps error: {}", err);
                                }

                                vcodec = VideoCodecType::H265;
                            }
                            Fmtp::Mpeg4(data) => {
                                let frame_data = FrameData::Audio {
                                    timestamp: 0,
                                    data: data.asc.clone(),
                                };

                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send asc error: {}", err);
                                }

                                audio_clock_rate = media.rtpmap.clock_rate;
                            }
                        }
                    }
                }

                if let Err(err) = sender.send(FrameData::MediaInfo {
                    media_info: MediaInfo {
                        audio_clock_rate,
                        video_clock_rate,

                        vcodec,
                    },
                }) {
                    log::error!("send media info error: {}", err);
                }
            }
            SubscribeType::PlayerHls => {}
            _ => {}
        }

        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StreamStatistics> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            log::error!("send_information of rtsp error: {}", err);
        }
    }
}
