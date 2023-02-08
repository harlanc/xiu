use {
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    clap::{value_parser, Arg, ArgGroup, Command},

    env_logger::{Builder, Env, Target},

    hls::server as hls_server,
    httpflv::server as httpflv_server,
    rtmp::{
        channels::channels::ChannelsManager,
        relay::{pull_client::PullClient, push_client::PushClient},
        rtmp::RtmpServer,
    },
    std::env,
    std::path::Path,
    tokio,
    tokio::signal,
    xiu::{
        config::{config, config::Config},
        logger::logger::{FileTarget, Rotate},
    },
};

//use application::logger::logger;
use hls::rtmp_event_processor::RtmpEventProcessor;

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Command::new("XIU")
        .bin_name("xiu")
        .version("0.1.0")
        .author("HarlanC <harlanc@foxmail.com>")
        .about("A live media server, hope you love it!!!")
        .arg(
            Arg::new("config_file_path")
                .long("config")
                .short('c')
                .value_name("path")
                .help("Specify the xiu server configuration file path.")
                .value_parser(value_parser!(String))
               // .conflicts_with_all(["rtmp", "httpflv", "hls"]),
        )
        .arg(
            Arg::new("rtmp")
                .long("rtmp")
                .short('p')
                .value_name("port")
                .help("Specify the RTMP listening port.")
                .default_value("1935")
                .value_parser(value_parser!(usize))
                //.conflicts_with("config_file_path"),
        )
        .arg(
            Arg::new("httpflv")
                .long("httpflv")
                .short('v')
                .value_name("port")
                .help("Specify the HTTP-FLV listening port.")
                .value_parser(value_parser!(usize))
                //.conflicts_with("config_file_path")
                .default_value("8080"),
        )
        .arg(
            Arg::new("hls")
                .long("hls")
                .short('s')
                .value_name("port")
                .help("Specify the HLS listening port.")
                .value_parser(value_parser!(usize))
                //.conflicts_with("config_file_path")
                .default_value("8081"),
        )
        .group(
            ArgGroup::new("vers")
                .args(["config_file_path", "rtmp"])
                .required(true),
        );



    let matches = cmd.get_matches();

    let config_file_path = &*matches.get_one("path").expect("msg");

    // if let Some(config_file_path) = matches.get_one("path") {}

    let config = config::load(config_file_path);
    match config {
        Ok(val) => {
            /*set log level*/

            // flexi_logger::Logger::try_with_env_or_str("info")?.start()?;
            // if let Some(log_config_value) = &val.log {
            //     flexi_logger::Logger::try_with_env_or_str(log_config_value.level.clone())?
            //         .start()?;
            // }
            if let Some(log_config_value) = &val.log {
                env::set_var("RUST_LOG", log_config_value.level.clone());
            } else {
                env::set_var("RUST_LOG", "info");
            }

            // let env = Env::default()
            //     .filter_or("MY_LOG_LEVEL", "trace")
            //     .write_style_or("MY_LOG_STYLE", "always");

            // Builder::from_env(env)
            //     .target(Target::Pipe(Box::new(FileTarget::new(
            //         Rotate::Minute,
            //         String::from("./logs"),
            //     ))))
            //     .init();

            env_logger::init();

            /*run the service*/
            let mut serivce = Service::new(val);
            serivce.run().await?;
        }
        _ => (),
    }

    // log::info!("log info...");
    // log::warn!("log warn...");
    // log::error!("log err...");
    // log::trace!("log trace...");
    // log::debug!("log debug...");

    signal::ctrl_c().await?;
    Ok(())
}

pub struct Service {
    cfg: Config,
}

impl Service {
    pub fn new(cfg: Config) -> Self {
        Service { cfg }
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
                    log::info!("start rtmp push client..");
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
                            log::error!("push client error {}\n", err);
                        }
                    });

                    channel.set_rtmp_push_enabled(true);
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
                    log::info!("start rtmp pull client from address: {}", address);
                    let mut pull_client = PullClient::new(
                        address,
                        channel.get_client_event_consumer(),
                        producer.clone(),
                    );

                    tokio::spawn(async move {
                        if let Err(err) = pull_client.run().await {
                            log::error!("pull client error {}\n", err);
                        }
                    });

                    channel.set_rtmp_pull_enabled(true);
                }
            }

            let listen_port = rtmp_cfg_value.port;
            let address = format!("0.0.0.0:{port}", port = listen_port);

            let mut rtmp_server = RtmpServer::new(address, producer);
            tokio::spawn(async move {
                if let Err(err) = rtmp_server.run().await {
                    //print!("rtmp server  error {}\n", err);
                    log::error!("rtmp server error: {}\n", err);
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
            let event_producer = channel.get_session_event_producer();

            tokio::spawn(async move {
                if let Err(err) = httpflv_server::run(event_producer, port).await {
                    //print!("push client error {}\n", err);
                    log::error!("httpflv server error: {}\n", err);
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

            let event_producer = channel.get_session_event_producer();
            let cient_event_consumer = channel.get_client_event_consumer();
            let mut rtmp_event_processor =
                RtmpEventProcessor::new(cient_event_consumer, event_producer);

            tokio::spawn(async move {
                if let Err(err) = rtmp_event_processor.run().await {
                    // print!("push client error {}\n", err);
                    log::error!("rtmp event processor error: {}\n", err);
                }
            });

            let port = hls_cfg_value.port;

            tokio::spawn(async move {
                if let Err(err) = hls_server::run(port).await {
                    //print!("push client error {}\n", err);
                    log::error!("hls server error: {}\n", err);
                }
            });
            channel.set_hls_enabled(true);
        }

        Ok(())
    }
}
