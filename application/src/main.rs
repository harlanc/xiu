use application::config::config;
use application::config::config::Config;
use rtmp::channels::channels::Channels;
use rtmp::session::server_session;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
//https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
use anyhow::Result;
use tokio;
#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load();
    match config {
        Ok(val) => {
            let mut rtmp_server = Service::new(val);
            rtmp_server.process_rtmp().await?;
        }
        _ => (),
    }
    Ok(())
}

pub struct Service {
    cfg: Config,
}

impl Service {
    pub fn new(cfg: Config) -> Self {
        Service { cfg: cfg }
    }
    async fn process_rtmp(&mut self) -> Result<()> {
        let mut channel = Channels::new();

        let rtmp = &self.cfg.rtmp;
        match rtmp {
            Some(rtmp_cfg) => {
                let listen_port = rtmp_cfg.port;
                let address = format!("0.0.0.0:{port}", port = listen_port);
                let socket_addr: &SocketAddr = &address.parse().unwrap();
                let mut listener = TcpListener::bind(socket_addr).await?;

                loop {
                    let (tcp_stream, addr) = listener.accept().await?;
                    tcp_stream.set_keepalive(Some(Duration::from_secs(30)))?;

                    let mut session =
                        server_session::ServerSession::new(tcp_stream, channel.get_event_producer(),Duration::from_secs(30));
                    tokio::spawn(async move { if let Err(err) = session.run().await {} });
                }
            }
            None => Ok(()),
        }
    }
}
