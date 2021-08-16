use {
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    application::config::{config, config::Config},
    hls::server as hls_server,
    httpflv::server,
    rtmp::{
        channels::channels::ChannelsManager,
        relay::{pull_client::PullClient, push_client::PushClient},
        session::server_session,
    },

    std::{env, net::SocketAddr},
    tokio,
    tokio::net::TcpListener,
};

use hls::rtmp_event_processor::RtmpEventProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let cfg_path = &args[1];
    let config = config::load(cfg_path);

    match config {
        Ok(val) => {
            let mut serivce = Service::new(val);
            serivce.run().await?;
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

    // fn process_hls(&mut self) -> Result<()> {
    //     let hls_cfg = &self.cfg.hls;
    //     match hls_cfg {
    //         Some(hls_cfg_value) => if hls_cfg_value.enabled {

    //             let hls_service = HlsService::new(consumer, event_producer)
    //         },
    //         None => {}
    //     }
    //     Ok(())
    // }

    async fn start_rtmp(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let producer = channel.get_session_event_producer();
        let event_producer = producer.clone();
        let rtmp = &self.cfg.rtmp;
        match rtmp {
            Some(rtmp_cfg) => {
                match rtmp_cfg.clone().push {
                    Some(push_cfg) => {
                        let address = format!(
                            "{ip}:{port}",
                            ip = push_cfg[0].address,
                            port = push_cfg[0].port
                        );

                        let mut push_client = PushClient::new(
                            address,
                            channel.get_client_event_consumer(),
                            producer.clone(),
                        );
                        tokio::spawn(async move {
                            if let Err(err) = push_client.run().await {
                                print!("push client error {}\n", err);
                            }
                        });

                        channel.set_push_enabled(true);
                    }
                    _ => {}
                }

                match rtmp_cfg.clone().pull {
                    Some(pull_cfg) => {
                        if pull_cfg.enabled {
                            let address =
                                format!("{ip}:{port}", ip = pull_cfg.address, port = pull_cfg.port);
                            let mut pull_client = PullClient::new(
                                address,
                                channel.get_client_event_consumer(),
                                producer.clone(),
                            );

                            tokio::spawn(async move {
                                if let Err(err) = pull_client.run().await {
                                    print!("pull client error {}\n", err);
                                }
                            });

                            channel.set_pull_enabled(true);
                        }
                    }
                    _ => {}
                }

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
            None => return Ok(()),
        }

        Ok(())
    }

    async fn start_httpflv(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let producer = channel.get_session_event_producer();
        let event_producer = producer.clone();
        tokio::spawn(async move {
            if let Err(err) = server::run(event_producer).await {
                print!("push client error {}\n", err);
            }
        });
        Ok(())
    }

    async fn start_hls(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let hls_cfg = &self.cfg.hls;

        match hls_cfg {
            Some(hls_cfg_value) => {
                if !hls_cfg_value.enabled {
                    return Ok(());
                }
            }
            None => return Ok(()),
        }

        let producer = channel.get_session_event_producer();
        let event_producer = producer.clone();
        let mut hls_service =
            RtmpEventProcessor::new(channel.get_client_event_consumer(), event_producer);

        tokio::spawn(async move {
            if let Err(err) = hls_service.run().await {
                print!("push client error {}\n", err);
            }
        });

        tokio::spawn(async move {
            if let Err(err) = hls_server::run().await {
                print!("push client error {}\n", err);
            }
        });
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        let mut channel = ChannelsManager::new();

        let producer = channel.get_session_event_producer();
        let event_producer = producer.clone();
        // tokio::spawn(async move {
        //     if let Err(err) = server::run(event_producer).await {
        //         print!("push client error {}\n", err);
        //     }
        // });

        self.start_httpflv(&mut channel).await?;
        self.start_hls(&mut channel).await?;

        let rtmp = &self.cfg.rtmp;
        match rtmp {
            Some(rtmp_cfg) => {
                match rtmp_cfg.clone().push {
                    Some(push_cfg) => {
                        let address = format!(
                            "{ip}:{port}",
                            ip = push_cfg[0].address,
                            port = push_cfg[0].port
                        );

                        let mut push_client = PushClient::new(
                            address,
                            channel.get_client_event_consumer(),
                            producer.clone(),
                        );
                        tokio::spawn(async move {
                            if let Err(err) = push_client.run().await {
                                print!("push client error {}\n", err);
                            }
                        });

                        channel.set_push_enabled(true);
                    }
                    _ => {}
                }

                match rtmp_cfg.clone().pull {
                    Some(pull_cfg) => {
                        if pull_cfg.enabled {
                            let address =
                                format!("{ip}:{port}", ip = pull_cfg.address, port = pull_cfg.port);
                            let mut pull_client = PullClient::new(
                                address,
                                channel.get_client_event_consumer(),
                                producer.clone(),
                            );

                            tokio::spawn(async move {
                                if let Err(err) = pull_client.run().await {
                                    print!("pull client error {}\n", err);
                                }
                            });

                            channel.set_pull_enabled(true);
                        }
                    }
                    _ => {}
                }

                let listen_port = rtmp_cfg.port;
                let address = format!("0.0.0.0:{port}", port = listen_port);
                let socket_addr: &SocketAddr = &address.parse().unwrap();
                let listener = TcpListener::bind(socket_addr).await?;

                let mut idx: u64 = 0;

                tokio::spawn(async move { channel.run().await });

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
