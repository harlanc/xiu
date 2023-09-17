use streamhub::define::StreamHubEventSender;

use super::session::WebRTCServerSession;

use super::http::define::http_method_name;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use streamhub::utils::Uuid;
use tokio::io::Error;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub struct WebRTCServer {
    address: String,
    event_producer: StreamHubEventSender,
    uuid_2_sessions: Arc<Mutex<HashMap<Uuid, Arc<Mutex<WebRTCServerSession>>>>>,
}

impl WebRTCServer {
    pub fn new(address: String, event_producer: StreamHubEventSender) -> Self {
        Self {
            address,
            event_producer,
            uuid_2_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("WebRTC server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            let session = Arc::new(Mutex::new(WebRTCServerSession::new(
                tcp_stream,
                self.event_producer.clone(),
            )));
            let uuid_2_sessions = self.uuid_2_sessions.clone();
            tokio::spawn(async move {
                let mut session_unlock = session.lock().await;
                if let Err(err) = session_unlock.run().await {
                    log::error!("session run error, err: {}", err);
                }

                if let Some(http_request_data) = &session_unlock.http_request_data {
                    let mut uuid_2_session_unlock = uuid_2_sessions.lock().await;

                    match http_request_data.method.as_str() {
                        http_method_name::POST => {
                            if let Some(uuid) = session_unlock.session_id {
                                uuid_2_session_unlock.insert(uuid, session.clone());
                            }
                        }
                        http_method_name::OPTIONS => {}
                        http_method_name::PATCH => {}
                        http_method_name::DELETE => {
                            if let Some(uuid) = session_unlock.session_id {
                                if let Some(session) = uuid_2_session_unlock.get(&uuid) {
                                    if let Err(err) =
                                        session.lock().await.close_peer_connection().await
                                    {
                                        log::error!("close peer connection failed: {}", err);
                                    } else {
                                        log::info!("close peer connection successfully.");
                                    }
                                    uuid_2_session_unlock.remove(&uuid);
                                } else {
                                    log::warn!("the session :{}  is not exited.", uuid);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
    }
}
