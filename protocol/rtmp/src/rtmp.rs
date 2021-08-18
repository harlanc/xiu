use super::channels::define::ChannelEventProducer;

use super::session::server_session;
use std::net::SocketAddr;
use tokio::io::Error;
use tokio::net::TcpListener;

pub struct RtmpServer {
    address: String,
    event_producer: ChannelEventProducer,
}

impl RtmpServer {
    pub fn new(address: String, event_producer: ChannelEventProducer) -> Self {
        Self {
            address,
            event_producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("Rtmp server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            //tcp_stream.set_keepalive(Some(Duration::from_secs(30)))?;

            let mut session =
                server_session::ServerSession::new(tcp_stream, self.event_producer.clone());
            tokio::spawn(async move {
                if let Err(err) = session.run().await {
                    log::info!(
                        "session exits, session_type: {}, app_name: {}, stream_name: {}",
                        session.common.session_type,
                        session.app_name,
                        session.stream_name
                    );
                    log::trace!("session err: {}", err);
                }
            });
        }
    }
}
