use {
    anyhow::Result, rtmp::channels::ChannelsManager, rtmp::session::client_session::ClientSession,
    rtmp::session::client_session::ClientType, std::env, tokio::net::TcpStream, tokio::signal,
    tokio::time::Duration,
};
#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let mut channel = ChannelsManager::new(None);
    let producer = channel.get_channel_event_producer();

    //pull the rtmp stream from rtmp server to local
    let ip = "127.0.0.1";
    let port = 1935;
    let address = format!("{ip}:{port}", ip = ip, port = port);

    let stream1 = TcpStream::connect(address.clone()).await?;
    let mut pull_client_session = ClientSession::new(
        stream1,
        ClientType::Play,
        String::from("live"),
        String::from("test"),
        producer.clone(),
    );

    // push the rtmp stream from local to remote rtmp server
    let stream2 = TcpStream::connect(address.clone()).await?;
    let mut push_client_session = ClientSession::new(
        stream2,
        ClientType::Publish,
        String::from("live"),
        String::from("test2?token=test"),
        producer.clone(),
    );

    push_client_session.subscribe(&pull_client_session);

    tokio::spawn(async move { channel.run().await });

    tokio::spawn(async move {
        if let Err(err) = pull_client_session.run().await {
            log::error!("pull_client_session as pull client run error: {}", err);
        }
    });

    tokio::time::sleep(Duration::from_secs(2)).await;

    tokio::spawn(async move {
        if let Err(err) = push_client_session.run().await {
            log::error!("push_client_session as push client run error: {}", err);
        }
    });

    signal::ctrl_c().await?;
    Ok(())
}
