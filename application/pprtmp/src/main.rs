use {
    anyhow::Result,
    clap::{value_parser, Arg, Command},
    rtmp::channels::ChannelsManager,
    rtmp::session::client_session::ClientSession,
    rtmp::session::client_session::ClientType,
    rtmp::utils::RtmpUrlParser,
    std::env,
    std::process::exit,
    tokio::net::TcpStream,
    tokio::signal,
    tokio::time::Duration,
};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let mut cmd = Command::new("pprtmp")
        .bin_name("pprtmp")
        .version("0.1.0")
        .author("HarlanC <harlanc@foxmail.com>")
        .about("pull and push rtmp!!!")
        .arg(
            Arg::new("pullrtmp")
                .long("pull_rtmp_url")
                .short('i')
                .value_name("path")
                .help("Specify the pull rtmp url.")
                .value_parser(value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("pushrtmp")
                .long("push_rtmp_url")
                .short('o')
                .value_name("path")
                .help("Specify the push rtmp url.")
                .value_parser(value_parser!(String))
                .required(true),
        );

    let args: Vec<String> = env::args().collect();
    if 1 == args.len() {
        cmd.print_help()?;
        return Ok(());
    }
    let matches = cmd.clone().get_matches();
    let pull_rtmp_url = matches.get_one::<String>("pullrtmp").unwrap().clone();
    let push_rtmp_url = matches.get_one::<String>("pushrtmp").unwrap().clone();

    let mut channel = ChannelsManager::new(None);
    let producer = channel.get_channel_event_producer();
    tokio::spawn(async move { channel.run().await });

    let mut pull_parser = RtmpUrlParser::new(pull_rtmp_url);
    if let Err(err) = pull_parser.parse_url() {
        log::error!("err: {}", err);
    }
    pull_parser.append_port(String::from("1935"));
    let stream1 = TcpStream::connect(pull_parser.raw_domain_name.clone()).await?;
    let mut pull_client_session = ClientSession::new(
        stream1,
        ClientType::Play,
        pull_parser.raw_domain_name,
        pull_parser.app_name.clone(),
        pull_parser.raw_stream_name,
        producer.clone(),
    );
    tokio::spawn(async move {
        if let Err(err) = pull_client_session.run().await {
            log::error!("pull_client_session as pull client run error: {}", err);
        }
    });

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut push_parser = RtmpUrlParser::new(push_rtmp_url);
    if let Err(err) = push_parser.parse_url() {
        log::error!("err: {}", err);
    }
    push_parser.append_port(String::from("1935"));
    // push the rtmp stream from local to remote rtmp server
    let stream2 = TcpStream::connect(push_parser.raw_domain_name.clone()).await?;
    let mut push_client_session = ClientSession::new(
        stream2,
        ClientType::Publish,
        push_parser.raw_domain_name,
        push_parser.app_name,
        push_parser.raw_stream_name,
        producer.clone(),
    );

    push_client_session.subscribe(pull_parser.app_name, pull_parser.stream_name);
    tokio::spawn(async move {
        if let Err(err) = push_client_session.run().await {
            log::error!("push_client_session as push client run error: {}", err);
            exit(0);
        }
    });

    signal::ctrl_c().await?;
    Ok(())
}
