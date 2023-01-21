# Introduction

This is a simple rtmp library for easy use and reading, you can build your own single rtmp server or a cluster .

# Examples

## Single Server

    use rtmp::channels::channels::ChannelsManager;
    use rtmp::rtmp::RtmpServer;

    #[tokio::main]

    async fn main() -> Result<()> {

        let mut channel = ChannelsManager::new();
        let producer = channel.get_session_event_producer();
    
        let listen_port = 1935;
        let address = format!("0.0.0.0:{port}", port = listen_port);
    
        let mut rtmp_server = RtmpServer::new(address, producer.clone());
        tokio::spawn(async move {
            if let Err(err) = rtmp_server.run().await {
                log::error!("rtmp server error: {}\n", err);
            }
        });
    
        tokio::spawn(async move { channel.run().await });

        signal::ctrl_c().await?;
        Ok(())
    }

## Cluster

    use rtmp::channels::channels::ChannelsManager;
    use rtmp::rtmp::RtmpServer;
    use rtmp::relay::{pull_client::PullClient, push_client::PushClient},

    #[tokio::main]

    async fn main() -> Result<()> {

        let mut channel = ChannelsManager::new();
        let producer = channel.get_session_event_producer();
        
        // push the rtmp stream from local to 192.168.0.2:1935
        let address = format!(
            "{ip}:{port}",
            ip = "192.168.0.2",
            port = 1935
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

        //pull the rtmp stream from 192.168.0.3:1935 to local
        let address = format!(
                  "{ip}:{port}",
                  ip = "192.168.0.3",
                  port = pull_cfg_value.port
              );
        log::info!("start rtmp pull client from address: {}", address);
        let mut pull_client = PullClient::new(
            address,
            channel.get_client_event_consumer(),
            producer.clone(),
        
        tokio::spawn(async move {
            if let Err(err) = pull_client.run().await {
                log::error!("pull client error {}\n", err);
            }
        });
        channel.set_rtmp_pull_enabled(true);

    
        // the local rtmp server
        let listen_port = 1935;
        let address = format!("0.0.0.0:{port}", port = listen_port);
    
        let mut rtmp_server = RtmpServer::new(address, producer.clone());
        tokio::spawn(async move {
            if let Err(err) = rtmp_server.run().await {
                log::error!("rtmp server error: {}\n", err);
            }
        });
    
        tokio::spawn(async move { channel.run().await });

        signal::ctrl_c().await?;
        Ok(())
    }

 For more detailed implementation please reference to [xiu server](https://github.com/harlanc/xiu/blob/master/application/xiu/src/main.rs)

     

# Version History
## v0.0.1
- support rtmp pushlish and play
## v0.0.2
- support rtmp relay pull and static push
## v0.0.3
- add amf0 functions 
## v0.0.4
- add timestamp for metadata 
## v0.0.5
- support complex handshake
## v0.0.6
- refactor some codes,update dependencies
## v0.0.7
- Fix bugs;
- add detail logs;
- improve subscriber id;
## v0.0.8
- Fix bugs;
## v0.0.9
- Support cache GOP;
- fix bugs;
- refactor handshake mod;
## v0.0.12
- Fix overflow error.[#17]
## v0.0.13
- add introductions and example codes in doc
## v0.0.14
- fix handshake error.[#23]
## v1.0.0
- Fix error chain.




