<p align="center" width="100%">
    <img width="38%" src="https://user-images.githubusercontent.com/10411078/149529602-7dcbaf26-55cd-4588-8989-206b76d32f07.png">
</p>


![XIU](https://img.shields.io/:XIU-blue.svg)[![crates.io](https://img.shields.io/crates/v/xiu.svg)](https://crates.io/crates/xiu)
[![crates.io](https://img.shields.io/crates/d/xiu.svg)](https://crates.io/crates/xiu)
![RTMP](https://img.shields.io/:RTMP-blue.svg)[![crates.io](https://img.shields.io/crates/v/rtmp.svg)](https://crates.io/crates/rtmp)
[![crates.io](https://img.shields.io/crates/d/rtmp.svg)](https://crates.io/crates/rtmp)
![HTTPFLV](https://img.shields.io/:HTTPFLV-blue.svg)[![crates.io](https://img.shields.io/crates/v/httpflv.svg)](https://crates.io/crates/httpflv)
[![crates.io](https://img.shields.io/crates/d/httpflv.svg)](https://crates.io/crates/httpflv)
![HLS](https://img.shields.io/:HLS-blue.svg)[![crates.io](https://img.shields.io/crates/v/hls.svg)](https://crates.io/crates/hls)
[![crates.io](https://img.shields.io/crates/d/hls.svg)](https://crates.io/crates/hls)
![FLV](https://img.shields.io/:FLV-blue.svg)[![crates.io](https://img.shields.io/crates/v/xflv.svg)](https://crates.io/crates/xflv)
[![crates.io](https://img.shields.io/crates/d/xflv.svg)](https://crates.io/crates/xflv)
![MPEGTS](https://img.shields.io/:MPEGTS-blue.svg)[![crates.io](https://img.shields.io/crates/v/xmpegts.svg)](https://crates.io/crates/xmpegts)
[![crates.io](https://img.shields.io/crates/d/xmpegts.svg)](https://crates.io/crates/xmpegts)
[![](https://app.travis-ci.com/harlanc/xiu.svg?branch=master)](https://app.travis-ci.com/github/harlanc/xiu)
[![](https://img.shields.io/discord/894502149764034560?logo=discord)](https://discord.gg/gS5wBRtpcB)
![wechat](https://img.shields.io/:微信-harlancc-blue.svg)
![qqgroup](https://img.shields.io/:QQ群-24893069-blue.svg)



[中文文档](https://github.com/harlanc/xiu/blob/master/README_CN.md)

Xiu is a simple,high performance and secure live media server written in pure Rust, it now supports popular live protocols like RTMP/HLS/HTTP-FLV, you can deploy it as a stand-alone server or a cluster using the relay feature.

## Features
- [x] Support multiple platforms(Linux/MacOS/Windows).
- [x] Support RTMP as a 
stand-alone server or cluster(RTMP relay).
   - [x] Support GOP cache which can be configured in the configuration file.
- [x] Support HTTP-FLV/HLS protocols(Transferred from RTMP).
- [x] Support configuring the service using command line or a configuration file.
- [x] Support HTTP API/Notifications.
  - [x] Support querying stream information.
  - [x] Support notify stream status.
- [x] Support token authentications.
- [ ] Support RTSP.

## Preparation
#### Install Rust and Cargo

[Document](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## Install and run 

There are two ways to install xiu :
 
 - Using cargo to install
 - Building from source


### Install using cargo

Issue the following command to install xiu:

    cargo install xiu
Start the service with the following command to get help:

    xiu -h
    A secure and easy to use live media server, hope you love it!!!

    Usage: xiu [OPTIONS] <--config <path>|--rtmp <port>>

    Options:
      -c, --config <path>   Specify the xiu server configuration file path.
      -r, --rtmp <port>     Specify the RTMP listening port(e.g.:1935).
      -f, --httpflv <port>  Specify the HTTP-FLV listening port(e.g.:8080).
      -s, --hls <port>      Specify the HLS listening port(e.g.:8081).
      -l, --log <level>     Specify the log level. [possible values: trace, debug, info, warn, error, debug]
      -h, --help            Print help
      -V, --version         Print version
    
### Build from souce

#### Clone Xiu

    git clone https://github.com/harlanc/xiu.git
    
use master branch
    
#### Build

    cd ./xiu/application/xiu
    cargo build --release
#### Run

    cd ./xiu/target/release
    ./xiu -h
    
## CLI

#### Instructions

You can use command line to configure the xiu server easily. You can specify to configure xiu using configuration file or from the command lines.

##### Configure using file

    xiu -c configuration_file_path

##### Configure using command line

    xiu -r 1935 -f 8080 -s 8081 -l info


#### How to Configure the configuration file

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
    [log.file]
    # write log to file or not（Writing logs to file or console cannot be satisfied at the same time）.
    enabled = true
    # set the rotate
    rotate = "hour" #[day,hour,minute]
    # set the path where the logs are saved
    path = "./logs"
    
### Configuration examples

I edit some configuration files under the following path which can be used directly:

    xiu/application/xiu/src/config

It contains the following 4 files:

    config_rtmp.toml //enable rtmp only
    config_rtmp_hls.toml //enable rtmp and hls
    config_rtmp_httpflv.toml //enable rtmp and httpflv
    config_rtmp_httpflv_hls.toml //enable all the 3 protocols

    

    
## Scenarios

##### Push

You can use two ways:

- Use OBS to push a live rtmp stream
- Or use FFmpeg to push a rtmp stream:
     
        ffmpeg -re -stream_loop -1 -i test.mp4 -c:a copy -c:v copy -f flv -flvflags no_duration_filesize rtmp://127.0.0.1:1935/live/test


##### Play

Use ffplay to play the rtmp/httpflv/hls live stream:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i http://localhost:8081/live/test.flv
    ffplay -i http://localhost:8080/live/test/test.m3u8
    
##### Relay - Static push

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


Use the above methods to push rtmp live stream to service 1, then the stream can be pushed to service 2 automatically, you can play the same live stream from both the two services:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test


    
##### Relay - Static pull

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

Use the above methods to push live stream to service 1, when you play the stream from serivce 2, it will pull the stream from service 1:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test
## Star History

[link](https://star-history.t9t.io/#harlanc/xiu)

## Thanks

 - [media_server](https://github.com/ireader/media-server.git)

## Others

Open issues if you have any problems. Star and pull requests are welcomed. Your stars can make this project go faster and further.
 


## v0.5.0
- Support rtmp gop number configuration.
- Support query stream information using HTTP api.

## v0.6.0
- Support notify stream status.
- Support HTTP API to kickoff clients.
- Add a http-server for testing http notify.
- Add a pull rtmp and push rtmp example: pprtmp.
- Fix some RTMP library bugs.
