pub mod errors;
use streamhub::{
    define::{
        DataSender, InformationSender, NotifyInfo, PublishType, PublisherInfo, StreamHubEvent,
        StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::StreamHubError,
    statistics::StatisticsStream,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::sync::Mutex;
use tokio::sync::{broadcast, oneshot};

use bytesio::bytesio::TNetIO;
use bytesio::bytesio::TcpIO;
use std::io::Read;
use std::{collections::HashMap, fs::File, sync::Arc};
use tokio::net::TcpStream;

use commonlib::define::http_method_name;
use commonlib::http::{parse_content_length, HttpRequest, HttpResponse};

use commonlib::http::Marshal as HttpMarshal;
use commonlib::http::Unmarshal as HttpUnmarshal;

use commonlib::auth::Auth;

use super::whep::handle_whep;
use super::whip::handle_whip;
use async_trait::async_trait;

use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;
use errors::SessionError;
use errors::SessionErrorValue;
use http::StatusCode;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection};

pub struct WebRTCServerSession {
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    event_sender: StreamHubEventSender,
    stream_handler: Arc<WebRTCStreamHandler>,

    pub session_id: Option<Uuid>,
    pub http_request_data: Option<HttpRequest>,
    pub peer_connection: Option<Arc<RTCPeerConnection>>,

    auth: Option<Auth>,
}

impl WebRTCServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
    ) -> Self {
        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            event_sender: event_producer,
            stream_handler: Arc::new(WebRTCStreamHandler::default()),
            session_id: None,
            http_request_data: None,
            peer_connection: None,
            auth,
        }
    }

    pub async fn close_peer_connection(&self) -> Result<(), SessionError> {
        if let Some(pc) = &self.peer_connection {
            pc.close().await?;
        }
        Ok(())
    }

    pub async fn run(
        &mut self,
        uuid_2_sessions: Arc<Mutex<HashMap<Uuid, Arc<Mutex<WebRTCServerSession>>>>>,
    ) -> Result<(), SessionError> {
        while self.reader.len() < 4 {
            let data = self.io.lock().await.read().await?;
            self.reader.extend_from_slice(&data[..]);
        }

        let mut remaining_data = self.reader.get_remaining_bytes();

        if let Some(content_length) = parse_content_length(std::str::from_utf8(&remaining_data)?) {
            while remaining_data.len() < content_length as usize {
                log::trace!(
                    "content_length: {} {}",
                    content_length,
                    remaining_data.len()
                );
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
                remaining_data = self.reader.get_remaining_bytes();
            }
        }

        let request_data = self.reader.extract_remaining_bytes();

        if let Some(http_request) = HttpRequest::unmarshal(std::str::from_utf8(&request_data)?) {
            //POST /whip?app=live&stream=test HTTP/1.1
            let eles: Vec<&str> = http_request.uri.path.splitn(2, '/').collect();
            let pars_map = &http_request.query_pairs;

            let request_method = http_request.method.as_str();
            if request_method == http_method_name::GET {
                let response = match http_request.uri.path.as_str() {
                    "/" => Self::gen_file_response("./index.html"),
                    "/whip.js" => Self::gen_file_response("./whip.js"),
                    "/whep.js" => Self::gen_file_response("./whep.js"),
                    _ => {
                        log::warn!(
                            "the http get path: {} is not supported.",
                            http_request.uri.path
                        );
                        return Ok(());
                    }
                };

                self.send_response(&response).await?;
                return Ok(());
            }

            if eles.len() < 2 || pars_map.get("app").is_none() || pars_map.get("stream").is_none() {
                log::error!(
                    "WebRTCServerSession::run the http path is not correct: {}",
                    http_request.uri.path
                );

                return Err(SessionError {
                    value: errors::SessionErrorValue::HttpRequestPathError,
                });
            }

            let t = eles[1];
            let app_name = pars_map.get("app").unwrap().clone();
            let stream_name = pars_map.get("stream").unwrap().clone();

            log::info!("1:{},2:{},3:{}", t, app_name, stream_name);

            match request_method {
                http_method_name::POST => {
                    let sdp_data = if let Some(body) = http_request.body.as_ref() {
                        body
                    } else {
                        return Err(SessionError {
                            value: errors::SessionErrorValue::HttpRequestEmptySdp,
                        });
                    };
                    self.session_id = Some(Uuid::new(RandomDigitCount::Zero));

                    let path = format!(
                        "{}?{}&session_id={}",
                        http_request.uri.path,
                        http_request.uri.query.as_ref().unwrap(),
                        self.session_id.unwrap()
                    );
                    let offer = RTCSessionDescription::offer(sdp_data.clone())?;

                    match t.to_lowercase().as_str() {
                        "whip" => {
                            if let Some(auth) = &self.auth {
                                auth.authenticate(&stream_name, &http_request.uri.query, false)?;
                            }
                            self.publish_whip(app_name, stream_name, path, offer)
                                .await?;
                        }
                        "whep" => {
                            if let Some(auth) = &self.auth {
                                auth.authenticate(&stream_name, &http_request.uri.query, true)?;
                            }
                            self.subscribe_whep(app_name, stream_name, path, offer)
                                .await?;
                        }
                        _ => {
                            log::error!(
                                "current path: {}, method: {}",
                                http_request.uri.path,
                                t.to_lowercase()
                            );
                            return Err(SessionError {
                                value: errors::SessionErrorValue::HttpRequestNotSupported,
                            });
                        }
                    }
                }
                http_method_name::OPTIONS => {
                    self.send_response(&Self::gen_response(http::StatusCode::OK))
                        .await?
                }
                http_method_name::PATCH => {}
                http_method_name::DELETE => {
                    if let Some(session_id) = pars_map.get("session_id") {
                        if let Some(uuid) = Uuid::from_str2(session_id) {
                            //stop the running session and delete it.
                            let mut uuid_2_sessions_unlock = uuid_2_sessions.lock().await;
                            if let Some(session) = uuid_2_sessions_unlock.get(&uuid) {
                                if let Err(err) = session.lock().await.close_peer_connection().await
                                {
                                    log::error!("close peer connection failed: {}", err);
                                } else {
                                    log::info!("close peer connection successfully.");
                                }
                                uuid_2_sessions_unlock.remove(&uuid);
                            } else {
                                log::warn!("the session :{}  is not exited.", uuid);
                            }
                        }
                    } else {
                        log::error!(
                            "the delete path does not contain session id: {}?{}",
                            http_request.uri.path,
                            http_request.uri.query.as_ref().unwrap()
                        );
                    }

                    match t.to_lowercase().as_str() {
                        "whip" => {
                            Self::unpublish_whip(
                                app_name,
                                stream_name,
                                self.get_publisher_info(),
                                self.event_sender.clone(),
                            )?;
                        }
                        "whep" => {}
                        _ => {
                            log::error!(
                                "current path: {}, method: {}",
                                http_request.uri.path,
                                t.to_lowercase()
                            );
                            return Err(SessionError {
                                value: errors::SessionErrorValue::HttpRequestNotSupported,
                            });
                        }
                    }

                    let status_code = http::StatusCode::OK;
                    let response = Self::gen_response(status_code);
                    self.send_response(&response).await?;
                }
                _ => {
                    log::warn!(
                        "WebRTCServerSession::unsupported method name: {}",
                        http_request.method
                    );
                }
            }

            self.http_request_data = Some(http_request);
        }

        Ok(())
    }

    async fn publish_whip(
        &mut self,
        app_name: String,
        stream_name: String,
        path: String,
        offer: RTCSessionDescription,
    ) -> Result<(), SessionError> {
        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            },
            result_sender: event_result_sender,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_sender.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let sender = event_result_receiver.await??;

        let response = match handle_whip(offer, sender.0, sender.1).await {
            Ok((session_description, peer_connection)) => {
                self.peer_connection = Some(peer_connection);

                let status_code = http::StatusCode::CREATED;
                let mut response = Self::gen_response(status_code);

                response
                    .headers
                    .insert("Content-Type".to_string(), "application/sdp".to_string());
                response.headers.insert("Location".to_string(), path);
                response.body = Some(session_description.sdp);

                response
            }
            Err(err) => {
                log::error!("handle whip err: {}", err);
                let status_code = http::StatusCode::SERVICE_UNAVAILABLE;
                Self::gen_response(status_code)
            }
        };

        self.send_response(&response).await
    }

    fn unpublish_whip(
        app_name: String,
        stream_name: String,
        publish_info: PublisherInfo,
        sender: StreamHubEventSender,
    ) -> Result<(), SessionError> {
        let unpublish_event = StreamHubEvent::UnPublish {
            identifier: StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            },
            info: publish_info,
        };

        if sender.send(unpublish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        Ok(())
    }

    async fn subscribe_whep(
        &mut self,
        app_name: String,
        stream_name: String,
        path: String,
        offer: RTCSessionDescription,
    ) -> Result<(), SessionError> {
        let subscriber_info = self.get_subscriber_info();

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::WebRTC {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
            },
            info: subscriber_info.clone(),
            result_sender: event_result_sender,
        };

        if self.event_sender.send(subscribe_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let receiver = event_result_receiver.await??.0.packet_receiver.unwrap();

        let (pc_state_sender, mut pc_state_receiver) = broadcast::channel(1);

        let response = match handle_whep(offer, receiver, pc_state_sender).await {
            Ok((session_description, peer_connection)) => {
                let pc_clone = peer_connection.clone();

                let app_name_out = app_name.clone();
                let stream_name_out = stream_name.clone();
                let subscriber_info_out = subscriber_info.clone();
                let sender_out = self.event_sender.clone();

                tokio::spawn(async move {
                    loop {
                        if let Ok(state) = pc_state_receiver.recv().await {
                            log::info!("state: {}", state);
                            match state {
                                RTCPeerConnectionState::Disconnected
                                | RTCPeerConnectionState::Failed => {
                                    if let Err(err) = pc_clone.close().await {
                                        log::error!("peer connection close error: {}", err);
                                    }
                                }
                                RTCPeerConnectionState::Closed => {
                                    if let Err(err) = Self::unsubscribe_whep(
                                        app_name_out,
                                        stream_name_out,
                                        subscriber_info_out,
                                        sender_out,
                                    ) {
                                        log::error!("unsubscribe whep error: {}", err);
                                    }
                                    break;
                                }
                                _ => {}
                            }
                        } else {
                            log::info!("recv");
                        }
                    }
                });

                self.peer_connection = Some(peer_connection);

                let status_code = http::StatusCode::CREATED;
                let mut response = Self::gen_response(status_code);
                response
                    .headers
                    .insert("Content-Type".to_string(), "application/sdp".to_string());
                response.headers.insert("Location".to_string(), path);
                response.body = Some(session_description.sdp);
                log::info!("before whep 1");
                response
            }
            Err(err) => {
                log::error!("handle whep err: {}", err);
                let status_code = http::StatusCode::SERVICE_UNAVAILABLE;
                Self::gen_response(status_code)
            }
        };
        self.send_response(&response).await
    }

    fn unsubscribe_whep(
        app_name: String,
        stream_name: String,
        subscriber_info: SubscriberInfo,
        sender: StreamHubEventSender,
    ) -> Result<(), SessionError> {
        let unsubscribe_event = StreamHubEvent::UnSubscribe {
            identifier: StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            },
            info: subscriber_info,
        };

        if sender.send(unsubscribe_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }
        Ok(())
    }

    fn get_subscriber_info(&self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::PlayerWebrtc,
            sub_data_type: streamhub::define::SubDataType::Packet,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        PublisherInfo {
            id,
            pub_type: PublishType::PushWebRTC,
            pub_data_type: streamhub::define::PubDataType::Both,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn gen_response(status_code: StatusCode) -> HttpResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            "".to_string()
        };

        let mut response = HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        response
            .headers
            .insert("Access-Control-Allow-Origin".to_owned(), "*".to_owned());
        response.headers.insert(
            "Access-Control-Allow-Headers".to_owned(),
            "content-type".to_owned(),
        );
        response
            .headers
            .insert("Access-Control-Allow-Method".to_owned(), "POST".to_owned());
        response
    }

    fn gen_file_response(file_path: &str) -> HttpResponse {
        let mut response = Self::gen_response(http::StatusCode::OK);

        let mut file = File::open(file_path).expect("Failed to open file");
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .expect("Failed to read file");

        let contents_str = String::from_utf8_lossy(&contents).to_string();

        response
            .headers
            .insert("Content-Type".to_string(), "text/html".to_string());
        response.body = Some(contents_str);

        response
    }

    async fn send_response(&mut self, response: &HttpResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;
        Ok(())
    }
}

#[derive(Default)]
pub struct WebRTCStreamHandler {
    sps: Mutex<Vec<u8>>,
    pps: Mutex<Vec<u8>>,
}

impl WebRTCStreamHandler {
    pub async fn set_sps(&self, sps: Vec<u8>) {
        *self.sps.lock().await = sps;
    }
    pub async fn set_pps(&self, pps: Vec<u8>) {
        *self.pps.lock().await = pps;
    }
}

#[async_trait]
impl TStreamHandler for WebRTCStreamHandler {
    async fn send_prior_data(
        &self,
        _data_sender: DataSender,
        _sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, _sender: InformationSender) {}
}
