# Introduction

This is a simple rtmp library for easy use and reading, you can build your own single rtmp server or a cluster .

# Examples

## Client

   You can use this library to push or pull RTMP streams , refer to the pprtmp example from xiu/application/pprtmp.

## Single Server

```rust
use rtmp::{
    relay::{pull_client::PullClient, push_client::PushClient},
    rtmp::RtmpServer,
};

 use {anyhow::Result, streamhub::StreamsHub};

fn start_single_server() {
    let mut stream_hub = StreamsHub::new(None);
    let sender = stream_hub.get_hub_event_sender();

    let listen_port = 1935;
    let address = format!("0.0.0.0:{port}", port = listen_port);

    let mut rtmp_server = RtmpServer::new(address, sender, 1);
    tokio::spawn(async move {
        if let Err(err) = rtmp_server.run().await {
            log::error!("rtmp server error: {}\n", err);
        }
    });

    tokio::spawn(async move { stream_hub.run().await });
}

#[tokio::main]

async fn main() -> Result<()> {
    start_single_server();
    //start_cluster();
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

## Cluster

```rust
use rtmp::{
    relay::{pull_client::PullClient, push_client::PushClient},
    rtmp::RtmpServer,
};

 use {anyhow::Result, streamhub::StreamsHub};

fn start_cluster() {
    let mut stream_hub = StreamsHub::new(None);
    let sender = stream_hub.get_hub_event_sender();

    // push the rtmp stream from local to 192.168.0.2:1935
    let address = format!("{ip}:{port}", ip = "192.168.0.2", port = 1935);

    let mut push_client = PushClient::new(
        address,
        stream_hub.get_client_event_consumer(),
        sender.clone(),
    );
    tokio::spawn(async move {
        if let Err(err) = push_client.run().await {
            log::error!("push client error {}\n", err);
        }
    });
    stream_hub.set_rtmp_push_enabled(true);

    //pull the rtmp stream from 192.168.0.3:1935 to local
    let address = format!("{ip}:{port}", ip = "192.168.0.3", port = "1935");
    log::info!("start rtmp pull client from address: {}", address);
    let mut pull_client = PullClient::new(
        address,
        stream_hub.get_client_event_consumer(),
        sender.clone(),
    );

    tokio::spawn(async move {
        if let Err(err) = pull_client.run().await {
            log::error!("pull client error {}\n", err);
        }
    });
    stream_hub.set_rtmp_pull_enabled(true);

    // the local rtmp server
    let listen_port = 1935;
    let address = format!("0.0.0.0:{port}", port = listen_port);

    let mut rtmp_server = RtmpServer::new(address, sender.clone(), 1);
    tokio::spawn(async move {
        if let Err(err) = rtmp_server.run().await {
            log::error!("rtmp server error: {}\n", err);
        }
    });

    tokio::spawn(async move { stream_hub.run().await });
}

#[tokio::main]

async fn main() -> Result<()> {
    start_cluster();
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

 For more detailed implementation please reference to [xiu server](https://github.com/harlanc/xiu/blob/master/application/xiu/src/main.rs)