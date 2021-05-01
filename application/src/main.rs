use {
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    application::config::{config, config::Config},
    rtmp::{
        application::push_client::PushClient, channels::channels::ChannelsManager,
        session::client_session, session::server_session,
    },
    std::net::SocketAddr,
    tokio,
    tokio::net::{TcpListener, TcpStream},
};

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
        let mut channel = ChannelsManager::new();

        let producer = channel.get_session_event_producer();
        let consumer = channel.get_channel_event_consumer();
        tokio::spawn(async move { channel.run().await });

        let rtmp = &self.cfg.rtmp;
        match rtmp {
            Some(rtmp_cfg) => {
                // match rtmp_cfg.clone().push {
                //     Some(push_cfg) => {
                //         let address = format!(
                //             "{ip}:{port}",
                //             ip = push_cfg[0].address,
                //             port = push_cfg[0].port
                //         );

                //         let mut push_client = PushClient::new(address, consumer, producer.clone());
                //         tokio::spawn(async move {
                //             if let Err(err) = push_client.run().await {
                //                 print!("push client error {}\n", err);
                //             }
                //         });
                //     }
                //     _ => {}
                // }
                let listen_port = rtmp_cfg.port;
                let address = format!("0.0.0.0:{port}", port = listen_port);
                let socket_addr: &SocketAddr = &address.parse().unwrap();
                let listener = TcpListener::bind(socket_addr).await?;

                let mut idx: u64 = 0;

                loop {
                    let (tcp_stream, _) = listener.accept().await?;
                    //tcp_stream.set_keepalive(Some(Duration::from_secs(30)))?;

                    let mut session =
                        server_session::ServerSession::new(tcp_stream, producer.clone(), idx);
                    tokio::spawn(async move {
                        if let Err(err) = session.run().await {
                            print!(
                                "session type: {}, id {}, session error {}\n",
                                session.session_type, session.session_id, err
                            );
                        }
                    });

                    idx = idx + 1;
                }
            }
            None => Ok(()),
        }
    }
}
