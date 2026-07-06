use super::session::define::rtsp_method_name;
use super::session::server_session::RtspServerSession;
use bytes::{Buf, Bytes};
use bytesio::bytesio::{TNetIO, TcpIO};
use commonlib::auth::Auth;
use commonlib::http::HttpResponse as RtspResponse;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use streamhub::define::StreamHubEventSender;
use tokio::io::Error;
use tokio::net::TcpListener;
pub struct RtspServer {
    address: String,
    event_producer: StreamHubEventSender,
    auth: Option<Auth>,
    white_list: Vec<String>,
    // 启用webrtc apm
    enable_apm: bool,
}

impl RtspServer {
    pub fn new(
        address: String,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        white_list: Vec<String>,
        enable_apm: bool,
    ) -> Self {
        Self {
            address,
            event_producer,
            auth,
            white_list,
            enable_apm,
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("Rtsp server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, remote_addr) = listener.accept().await?;
            let _ = tcp_stream.set_linger(Some(Duration::from_millis(0)));

            if self.white_list.len() > 0 {
                let remote_addr_str = remote_addr.ip().to_string();
                if !self.white_list.contains(&remote_addr_str) {
                    log::warn!("Remote socket {remote_addr_str} is not in white_list");
                    continue;
                }
            }
            let mut session = RtspServerSession::new(
                tcp_stream,
                self.event_producer.clone(),
                self.auth.clone(),
                self.enable_apm,
            );
            tokio::spawn(async move {
                if let Err(err) = session.run().await {
                    let session_id = if let Some(id) = session.session_id {
                        id.to_string()
                    } else {
                        "none".to_string()
                    };
                    log::info!(
                        "session run exit: session id: {} session type: {} , err: {}",
                        session_id,
                        session.session_type,
                        err
                    );

                    if !session.is_normal_exit {
                        if let Some(identifier) = session.stream_identifier.clone() {
                            match session.exit(identifier) {
                                Err(err) => {
                                    log::error!(
                                        "session exit error: session id: {} session type: {}, error info: {}",
                                        session_id,
                                        session.session_type,
                                        err
                                    );
                                }
                                Ok(()) => {
                                    log::info!(
                                        "session exit successfully: session id: {} session type: {} ",
                                        session_id,
                                        session.session_type,
                                    );
                                }
                            }
                        }
                    }
                }
                let sid = session
                    .session_id
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or("".to_owned());
                // let sio = session.io.clone();
                log::info!(
                    "Dropping session {sid} type {} and shutting down io",
                    session.session_type
                );
                // drop(session);
                // if let Some(lock) = Arc::into_inner(sio) {
                //     let net_io: Box<dyn TNetIO + Send + Sync> = lock.into_inner();
                //     if let Ok(tcp_io) = net_io.into_any().downcast::<TcpIO>() {
                //         let _ = tcp_io.shut_down();
                //     }
                // }
            });
        }
    }
}

// pub trait MyToAny: 'static {
//     fn as_any(&self) -> &dyn std::any::Any;
// }

// impl<T: 'static> MyToAny for T {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
// }
