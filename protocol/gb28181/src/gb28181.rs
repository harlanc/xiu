use super::session::GB28181ServerSession;
use std::net::SocketAddr;
use streamhub::define::StreamHubEventSender;
use tokio::io::Error;
use tokio::net::TcpListener;

pub struct GB28181Server {
    address: String,
    event_producer: StreamHubEventSender,
}

impl GB28181Server {
    pub fn new(address: String, event_producer: StreamHubEventSender) -> Self {
        Self {
            address,
            event_producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("GB28181 server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            if let Ok(mut session) =
                GB28181ServerSession::new(tcp_stream, self.event_producer.clone())
            {
                tokio::spawn(async move {
                    if let Err(err) = session.run().await {
                        log::error!("session run error, err: {}", err);
                    }
                });
            }
        }
    }
}
