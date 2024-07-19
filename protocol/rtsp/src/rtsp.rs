use streamhub::define::StreamHubEventSender;

use super::session::RtspServerSession;
use commonlib::auth::Auth;
use std::net::SocketAddr;
use tokio::io::Error;
use tokio::net::TcpListener;

pub struct RtspServer {
    address: String,
    event_producer: StreamHubEventSender,
    auth: Option<Auth>,
}

impl RtspServer {
    pub fn new(address: String, event_producer: StreamHubEventSender, auth: Option<Auth>) -> Self {
        Self {
            address,
            event_producer,
            auth,
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("Rtsp server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            let mut session =
                RtspServerSession::new(tcp_stream, self.event_producer.clone(), self.auth.clone());
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
            });
        }
    }
}
