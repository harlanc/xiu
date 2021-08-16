use {
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    application::config::{config, config::Config},
    hls::server as hls_server,
    httpflv::server,

    rtmp::{
        channels::channels::ChannelsManager,
        relay::{pull_client::PullClient, push_client::PushClient},
        rtmp::RtmpServer,
    },
    std::env,
    tokio,

    tokio::signal,
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

    signal::ctrl_c().await?;
    Ok(())
}

pub struct Service {
    cfg: Config,
}

impl Service {
    pub fn new(cfg: Config) -> Self {
        Service { cfg: cfg }
    }

    async fn run(&mut self) -> Result<()> {
        let mut channel = ChannelsManager::new();

        self.start_httpflv(&mut channel).await?;
        self.start_hls(&mut channel).await?;
        self.start_rtmp(&mut channel).await?;

        tokio::spawn(async move { channel.run().await });

        Ok(())
    }

    async fn start_rtmp(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let rtmp_cfg = &self.cfg.rtmp;

        if let Some(rtmp_cfg_value) = rtmp_cfg {
            if !rtmp_cfg_value.enabled {
                return Ok(());
            }

            let producer = channel.get_session_event_producer();

            /*static push */
            if let Some(push_cfg_values) = &rtmp_cfg_value.push {
                for push_value in push_cfg_values {
                    if !push_value.enabled {
                        continue;
                    }
                    let address = format!(
                        "{ip}:{port}",
                        ip = push_value.address,
                        port = push_value.port
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
            }
            /*static pull*/
            if let Some(pull_cfg_value) = &rtmp_cfg_value.pull {
                if pull_cfg_value.enabled {
                    let address = format!(
                        "{ip}:{port}",
                        ip = pull_cfg_value.address,
                        port = pull_cfg_value.port
                    );
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

            let listen_port = rtmp_cfg_value.port;
            let address = format!("0.0.0.0:{port}", port = listen_port);

            let mut rtmp_server = RtmpServer::new(address, producer.clone());
            tokio::spawn(async move {
                if let Err(err) = rtmp_server.run().await {
                    print!("rtmp server  error {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_httpflv(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let httpflv_cfg = &self.cfg.httpflv;

        if let Some(httpflv_cfg_value) = httpflv_cfg {
            if !httpflv_cfg_value.enabled {
                return Ok(());
            }
            let port = httpflv_cfg_value.port;
            let event_producer = channel.get_session_event_producer().clone();

            tokio::spawn(async move {
                if let Err(err) = server::run(event_producer, port).await {
                    print!("push client error {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_hls(&mut self, channel: &mut ChannelsManager) -> Result<()> {
        let hls_cfg = &self.cfg.hls;

        if let Some(hls_cfg_value) = hls_cfg {
            if !hls_cfg_value.enabled {
                return Ok(());
            }

            let event_producer = channel.get_session_event_producer().clone();
            let mut rtmp_event_processor =
                RtmpEventProcessor::new(channel.get_client_event_consumer(), event_producer);

            tokio::spawn(async move {
                if let Err(err) = rtmp_event_processor.run().await {
                    print!("push client error {}\n", err);
                }
            });

            tokio::spawn(async move {
                if let Err(err) = hls_server::run().await {
                    print!("push client error {}\n", err);
                }
            });
        }

        Ok(())
    }
}
