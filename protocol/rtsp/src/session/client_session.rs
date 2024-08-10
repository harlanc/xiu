use super::define;
use super::define::ClientSessionType;

use crate::global_trait::Marshal;
use crate::global_trait::Unmarshal;
use crate::rtsp_codec;

use crate::rtsp_transport::CastType;

use super::server_session::InterleavedBinaryData;
use commonlib::http::HttpRequest as RtspRequest;
use commonlib::http::HttpResponse as RtspResponse;
use commonlib::http::Marshal as RtspMarshal;
use commonlib::http::Unmarshal as RtspUnmarshal;
use commonlib::http::Uri;
use streamhub::define::SubscriberInfo;

use crate::rtp::RtpPacket;

use crate::rtsp_codec::RtspCodecInfo;
use crate::rtsp_track::RtspTrack;
use crate::rtsp_track::TrackType;
use crate::rtsp_transport::ProtocolType;
use crate::rtsp_transport::RtspTransport;

use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;

use super::errors::SessionError;
use super::errors::SessionErrorValue;

use tokio::sync::oneshot;

use crate::rtp::errors::UnPackerError;
use crate::sdp::Sdp;

use super::define::rtsp_method_name;

use bytesio::bytesio::TNetIO;
use bytesio::bytesio::TcpIO;

use std::collections::HashMap;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::define::USER_AGENT;

use streamhub::{
    define::{
        FrameData, NotifyInfo, PublishType, PublisherInfo, StreamHubEvent, StreamHubEventSender,
        SubscribeType,
    },
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::server_session::RtspStreamHandler;

use bytesio::bytesio::new_udpio_pair;

pub struct RtspClientSession {
    address: String,
    stream_name: String,

    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    protocol_type: ProtocolType,
    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,
    pub client_type: super::define::ClientSessionType,
    cseq: u16,
    stream_handler: Arc<RtspStreamHandler>,

    event_producer: StreamHubEventSender,
    pub is_running: Arc<AtomicBool>,
}

impl RtspClientSession {
    pub async fn new(
        address: String,
        stream_name: String,
        protocol_type: ProtocolType,
        event_producer: StreamHubEventSender,
        client_type: ClientSessionType,
    ) -> Result<Self, SessionError> {
        let stream = TcpStream::connect(address.clone()).await?;

        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Ok(Self {
            address,
            stream_name,
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            protocol_type,
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            client_type,
            event_producer,

            cseq: 1,

            stream_handler: Arc::new(RtspStreamHandler::new()),
            is_running: Arc::new(AtomicBool::new(true)),
        })
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    pub async fn run(&mut self) -> Result<(), SessionError> {
        self.send_options().await?;

        match self.client_type {
            ClientSessionType::Pull => {
                self.send_describe().await?;
                self.send_setup().await?;
                self.send_play().await?;
            }
            ClientSessionType::Push => {
                self.send_announce().await?;
                self.send_setup().await?;
                self.send_record().await?;
            }
        }

        while self.is_running.load(std::sync::atomic::Ordering::Acquire) {
            while self.reader.len() < 4 {
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
            }

            if let Ok(Some(a)) = InterleavedBinaryData::new(&mut self.reader) {
                if self.reader.len() < a.length as usize {
                    let data = self.io.lock().await.read().await?;
                    self.reader.extend_from_slice(&data[..]);
                }
                self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                    .await?;
            }
        }

        self.send_teardown().await?;

        Ok(())
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
    async fn send_options(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_options");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::OPTIONS, uri_path);
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::OPTIONS).await
    }

    async fn send_announce(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_announce");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::ANNOUNCE, uri_path);
        self.send_resquest(&request).await
    }

    async fn send_describe(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_describe");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::DESCRIBE, uri_path);
        request
            .headers
            .insert("Accept".to_string(), "application/sdp".to_string());
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::DESCRIBE).await
    }

    async fn send_setup(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_setup");
        let sdp_medias = self.sdp.medias.clone();

        for media in sdp_medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                log::error!("cannot get media control!!");
                String::from("")
            };

            let uri_path = format!(
                "rtsp://{}/{}/{}",
                self.address, self.stream_name, media_control
            );

            let mut request = self.gen_request(rtsp_method_name::SETUP, uri_path);

            match self.protocol_type {
                ProtocolType::TCP => {
                    let kv: Vec<&str> = media_control.trim().splitn(2, '=').collect();

                    let mut media_transport = RtspTransport::default();
                    if let Ok(interleaved_idx) = kv[1].parse::<u8>() {
                        media_transport.interleaved =
                            Some([interleaved_idx * 2, interleaved_idx * 2 + 1]);
                    } else {
                        log::error!("cannot get interleaved_idx: {}", kv[1]);
                    }

                    media_transport.protocol_type = ProtocolType::TCP;
                    media_transport.cast_type = CastType::Unicast;
                    request
                        .headers
                        .insert("Transport".to_string(), media_transport.marshal());

                    if media.media_type == "audio" {
                        if let Some(track) = self.tracks.get_mut(&TrackType::Audio) {
                            track.transport.interleaved = media_transport.interleaved;
                        }
                    } else if media.media_type == "video" {
                        if let Some(track) = self.tracks.get_mut(&TrackType::Video) {
                            track.transport.interleaved = media_transport.interleaved;
                        }
                    }
                }
                ProtocolType::UDP => {
                    if let Some((socket_rtp, socket_rtcp)) = new_udpio_pair().await {
                        let media_transport = RtspTransport {
                            protocol_type: ProtocolType::UDP,
                            cast_type: CastType::Unicast,
                            client_port: Some([
                                socket_rtp.get_local_port().unwrap(),
                                socket_rtcp.get_local_port().unwrap(),
                            ]),
                            ..Default::default()
                        };

                        request
                            .headers
                            .insert("Transport".to_string(), media_transport.marshal());

                        if media.media_type == "audio" {
                            if let Some(track) = self.tracks.get_mut(&TrackType::Audio) {
                                let box_rtp_io: Box<dyn TNetIO + Send + Sync> =
                                    Box::new(socket_rtp);
                                track.rtp_receive_loop(box_rtp_io).await;

                                let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                    Arc::new(Mutex::new(Box::new(socket_rtcp)));
                                track.rtcp_receive_loop(box_rtcp_io).await;
                            }
                        } else if media.media_type == "video" {
                            if let Some(track) = self.tracks.get_mut(&TrackType::Video) {
                                let box_rtp_io: Box<dyn TNetIO + Send + Sync> =
                                    Box::new(socket_rtp);
                                track.rtp_receive_loop(box_rtp_io).await;

                                let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                    Arc::new(Mutex::new(Box::new(socket_rtcp)));
                                track.rtcp_receive_loop(box_rtcp_io).await;
                            }
                        }
                    }
                }
            }

            self.send_resquest(&request).await?;
            self.receive_response(rtsp_method_name::SETUP).await?;
        }
        Ok(())
    }

    async fn send_play(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_play");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::PLAY, uri_path);
        request
            .headers
            .insert("Range".to_string(), "npt=0.000".to_string());

        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::PLAY).await?;

        Ok(())
    }

    async fn send_record(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_record");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::RECORD, uri_path);
        request
            .headers
            .insert("Transport".to_string(), "application/sdp".to_string());
        self.send_resquest(&request).await
    }

    async fn send_teardown(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_teardown");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::TEARDOWN, uri_path);
        self.send_resquest(&request).await?;
        self.exit()
    }

    fn gen_request(&mut self, method_name: &str, uri_path: String) -> RtspRequest {
        let uri = Uri::unmarshal(&uri_path).unwrap();

        let mut request = RtspRequest {
            method: method_name.to_string(),
            uri,
            version: "RTSP/1.0".to_string(),
            ..Default::default()
        };

        request
            .headers
            .insert("CSeq".to_string(), self.cseq.to_string());
        self.cseq += 1;
        request
            .headers
            .insert("User-Agent".to_string(), USER_AGENT.to_string());

        if let Some(session_id) = self.session_id {
            request
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }

        request
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::RtspRelay,
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
            pub_type: PublishType::RtspRelay,
            pub_data_type: streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
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
                    log::info!("video codec info: {:?}", codec_info);
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn send_resquest(&mut self, request: &RtspRequest) -> Result<(), SessionError> {
        self.writer.write(request.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }

    async fn receive_response(&mut self, method_name: &str) -> Result<(), SessionError> {
        let data = self.io.lock().await.read().await?;
        self.reader.extend_from_slice(&data[..]);

        let mut retry_count = 0;
        let rtsp_response;

        loop {
            let data = self.reader.get_remaining_bytes();
            if let Some(rtsp_response_data) = RtspResponse::unmarshal(std::str::from_utf8(&data)?) {
                // TCP packet sticking issue, if have content_length in header.
                // should check the body
                if let Some(content_length) =
                    rtsp_response_data.get_header(&String::from("Content-Length"))
                {
                    if let Ok(uint_num) = content_length.parse::<usize>() {
                        if rtsp_response_data.body.is_none()
                            || uint_num > rtsp_response_data.body.clone().unwrap().len()
                        {
                            if retry_count >= 5 {
                                log::error!(
                                    "corrupted rtsp message={}",
                                    std::str::from_utf8(&data)?
                                );
                                return Ok(());
                            }
                            retry_count += 1;
                            let data_recv = self.io.lock().await.read().await?;
                            self.reader.extend_from_slice(&data_recv[..]);
                            continue;
                        }
                    }
                }
                rtsp_response = rtsp_response_data;
                self.reader.extract_remaining_bytes();
                break;
            } else {
                log::error!("corrupted rtsp message={}", std::str::from_utf8(&data)?);
                return Ok(());
            }
        }

        if rtsp_response.status_code != http::StatusCode::OK {
            log::error!("rtsp response error: {}", rtsp_response.marshal());
            return Err(SessionError {
                value: SessionErrorValue::RtspResponseStatusError,
            });
        }

        match method_name {
            rtsp_method_name::OPTIONS => {
                if let Some(public) = rtsp_response.get_header(&"Public".to_string()) {
                    log::info!("support methods: {}", public);
                }
            }
            rtsp_method_name::ANNOUNCE => {}
            rtsp_method_name::DESCRIBE => {
                if let Some(request_body) = &rtsp_response.body {
                    if let Some(sdp) = Sdp::unmarshal(request_body) {
                        self.sdp = sdp.clone();
                        self.stream_handler.set_sdp(sdp).await;

                        self.new_tracks()?;

                        let (event_result_sender, event_result_receiver) = oneshot::channel();
                        let identifier = StreamIdentifier::Rtsp {
                            stream_path: self.stream_name.clone(),
                        };

                        let publish_event = StreamHubEvent::Publish {
                            identifier,
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
                            rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(
                                move |packet: RtpPacket| {
                                    let rtcp_channel_in = Arc::clone(&rtcp_channel);
                                    Box::pin(async move {
                                        rtcp_channel_in.lock().await.on_packet(packet);
                                    })
                                },
                            ));
                        }
                    }
                }
            }
            rtsp_method_name::SETUP => {
                if self.session_id.is_none() {
                    if let Some(session_id) = rtsp_response.get_header(&"Session".to_string()) {
                        self.session_id = Uuid::from_str2(session_id);
                    }
                }

                if let Some(transport_str) = rtsp_response.get_header(&"Transport".to_string()) {
                    log::info!("setup response: transport {}", transport_str);
                }
            }
            rtsp_method_name::PLAY => {}
            rtsp_method_name::RECORD => {}
            _ => {}
        }
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), SessionError> {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: self.stream_name.clone(),
        };
        let event = match self.client_type {
            define::ClientSessionType::Push => StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            },
            define::ClientSessionType::Pull => StreamHubEvent::UnPublish {
                identifier,
                info: self.get_publisher_info(),
            },
        };

        let event_json_str = serde_json::to_string(&event).unwrap();

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                log::error!("session exit: send event error: {err} for event: {event_json_str}");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                log::info!("session exit: send event success: {event_json_str}");
                Ok(())
            }
        }
    }
}
