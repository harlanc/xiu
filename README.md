# Xiu
**Xiu is a live server written by Rust.**


## Functionalities

- [x] RTMP
  - [x] publish and play 
  - [x] relay: static push
  - [x] relay: static pull
- [x] HTTPFLV
- [x] HLS


## Dev Environment Establish
#### Install Rust and Cargo

[Document](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## Install and run 

There are two ways to install xiu :
 
 - Using cargo to install
 - Building from source


### Install using cargo

Issue the following command to install xiu:

    cargo install xiu
Start the service with the following command:

    xiu configuration_file_path/confit.toml
    
### Build from souce

#### Clone Xiu

    git clone https://github.com/harlanc/xiu.git
    
use master branch
    
#### Build

    cd ./xiu/application/xiu
    cargo build --release
#### Run

    cd ./xiu/target/release
    ./xiu config.toml
    
## Configurations

##### RTMP
    [rtmp]
    enabled = true
    port = 1935

    # pull streams from other server node.
    [rtmp.pull]
    enabled = false
    address = "192.168.0.1"
    port = 1935

    # push streams to other server node.
    [[rtmp.push]]
    enabled = true
    address = "localhost"
    port = 1936
    [[rtmp.push]]
    enabled = true
    address = "192.168.0.3"
    port = 1935
    
##### HTTPFLV

    [httpflv]
    # true or false to enable or disable the feature
    enabled = true
    # listening port
    port = 8081

##### HLS
    [hls]
    # true or false to enable or disable the feature
    enabled = true
    # listening port
    port = 8080

##### Log

    [log]
    level = "info"

    

    
## Scenarios

##### Push

Use OBS to push a live rtmp stream.


##### Play

Use ffplay to play rtmp/httpflv/hls live stream:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i http://localhost:8081/live/test.flv
    ffplay -i http://localhost:8080/live/test/test.m3u8
    
##### Relay static push

The configuration files are as follows:

The configuration file of Service 1 named config.toml:

    [rtmp]
    enabled = true
    port = 1935
    [[rtmp.push]]
    enabled = true
    address = "localhost"
    port = 1936
    
The configuration file of Service 2 named config_push.toml:

    [rtmp]
    enabled = true
    port = 1936

Run the 2 services:

    ./xiu config.toml
    ./xiu config_push.toml


Use Obs to push live stream to service 1, then the stream can be pushed to service 2 automatically, you can play the same live stream from both the two services:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test


    
##### Relay static pull

The configuration file are as follows:

The configuration file of Service 1 named config.toml:

    [rtmp]
    enabled = true
    port = 1935

 
The configuration file of Service 2 named config_pull.toml:

    [rtmp]
    enabled = true
    port = 1936
    [rtmp.pull]
    enabled = false
    address = "localhost"
    port = 1935

Run the 2 services:

    ./xiu config.toml
    ./xiu config_pull.toml

Use obs to push live stream to service 1, when you play the stream from serivce 2, it will pull the stream from service 1:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test
## Star History

[link](https://star-history.t9t.io/#harlanc/xiu)

## Thanks

 - [media_server](https://github.com/ireader/media-server.git)

## Others

You can open issues if have any problems. Star and pull requests are welcomed.
 
