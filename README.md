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

#### OS Support

-  CentOS 7
-  MaxOS

#### Install Rust and Cargo

[Document](https://doc.rust-lang.org/cargo/getting-started/installation.html)

#### Clone Xiu

    git clone https://github.com/harlanc/xiu.git
    
use master branch
    
#### Build

    cd ./xiu/application/xiu
    
    cargo build
    
#### Run

    cd ./xiu/target/debug
    
    ./application config.toml
    
#### Push

Use OBS to push a live rtmp stream.


#### Play

Use ffplay to play rtmp live stream:

    ffplay -i rtmp://localhost:1935/live/test
    
#### Relay static push

The configuration file is as follows (now only test on local machine):

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

    ./application config.toml
    ./application config_push.toml


Use Obs to push live stream to service 1, then the stream can be pushed to service 2 automatically, you can play the same live stream from both the two services:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test


    
#### Relay pull

The configuration file is as follows (now only test on local machine):

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

    ./application config.toml
    ./application config_pull.toml

Use obs to push live stream to service 1, when you play the stream from serivce 2, it will pull the stream from service 1:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test
## Star History

[link](https://star-history.t9t.io/#harlanc/xiu)
