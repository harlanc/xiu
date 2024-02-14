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

     

# Version History
## v0.0.1
- Support rtmp pushlish and play
## v0.0.2
- Support rtmp relay pull and static push
## v0.0.3
- Add amf0 functions 
## v0.0.4
- Add timestamp for metadata 
## v0.0.5
- Support complex handshake
## v0.0.6
- Refactor some codes,update dependencies
## v0.0.7
- Fix bugs;
- Add detail logs;
- Improve subscriber id;
## v0.0.8
- Fix bugs;
## v0.0.9
- Support cache GOP;
- Fix bugs;
- Refactor handshake mod;
## v0.0.12
- Fix overflow error.[#17]
## v0.0.13
- Add introductions and example codes in doc
## v0.0.14
- Fix handshake error.[#23]
## v0.1.0
- Update RTMP library version.
## v0.2.0
- Support audio and video information statistics.
## v0.3.0
- Support notify stream status.
- Add HTTP API to kickoff clients.
- Fix some client session bugs.
## v0.3.1
- Fix error that cannot receive rtmp stream pushed from GStreamer.
- Reference xflv new version v0.2.1.
- Fix RTMP examples in README.
## v0.4.0
- Reference bytesio v0.3.0.
- Support transferring from rtsp to rtmp.
- Do some refactoring.
## 0.4.2
- Reference streamhub new version v0.1.2.
## v0.5.0
- Remove no used "\n" for error message.
- Receive and process sub event result.
- Fix RTMP chunk parse error.
- Fix RTMP chunks are uncompressed in packetizer mod.
- Fix err: when encountering an unknown RTMP message type, it should be skipped rather than returning an error.
- Support remuxing from WHIP to rtmp.
## 0.6.0
- Support auth.
## 0.6.1
- Fix RTMP build error.




