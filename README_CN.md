<p align="center" width="100%">
    <img width="38%" src="https://user-images.githubusercontent.com/10411078/149529602-7dcbaf26-55cd-4588-8989-206b76d32f07.png">
</p>


![XIU](https://img.shields.io/:XIU-blue.svg)[![crates.io](https://img.shields.io/crates/v/xiu.svg)](https://crates.io/crates/xiu)
[![crates.io](https://img.shields.io/crates/d/xiu.svg)](https://crates.io/crates/xiu)
![RTMP](https://img.shields.io/:RTMP-blue.svg)[![crates.io](https://img.shields.io/crates/v/rtmp.svg)](https://crates.io/crates/rtmp)
[![crates.io](https://img.shields.io/crates/d/rtmp.svg)](https://crates.io/crates/rtmp)
![RTSP](https://img.shields.io/:RTSP-blue.svg)[![crates.io](https://img.shields.io/crates/v/xrtsp.svg)](https://crates.io/crates/xrtsp)
[![crates.io](https://img.shields.io/crates/d/xrtsp.svg)](https://crates.io/crates/xrtsp)
![WEBRTC](https://img.shields.io/:WEBRTC-blue.svg)[![crates.io](https://img.shields.io/crates/v/xwebrtc.svg)](https://crates.io/crates/xwebrtc)
[![crates.io](https://img.shields.io/crates/d/xwebrtc.svg)](https://crates.io/crates/xwebrtc)
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


XIU是用纯Rust开发的一款简单和安全的流媒体服务器，目前支持的流媒体协议包括RTMP[cluster]/RTSP/WebRTC[Whip/Whep]/HLS/HTTPFLV。

## 功能

- [x] 支持多平台（Linux/Mac/Windows）
- [x] 支持RTMP
  - [x] 支持发布和订阅H264/AAC 直播流；
  - [x] 支持秒开（Gop cache）；
  - [x] 支持转换到HLS/HTTP-FLV协议； 
  - [x] 支持部署集群；
- [x] 支持RTSP
   - [x] 支持通过TCP（Interleaved）和UDP发布或订阅H.265/H.264/AAC流；
   - [x] 支持转换到RTMP/HLS/HTTP-FLV协议；
- [x] 支持WebRTC（Whip/Whep）
   - [x] 支持使用Whip发布rtc流；
   - [x] 支持使用Whep订阅rtc流；
   - [x] 支持转换到RTMP/HLS/HTTP-FLV协议；
- [x] 支持订阅HLS/HTTPFLV直播流
- [x] 支持命令行或者配置文件配置服务
- [x] 支持HTTP API/notify
    - [x] 支持查询流信息；
    - [x] 支持流事件通知；
- [x] 支持token鉴权
- [x] 支持把直播流录制成HLS协议(m3u8+ts)文件

## 准备工作
#### 安装 Rust and Cargo


[Document](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## 安装和运行

有两种方式来安装xiu：
 
 - 直接用cargo来安装
 - 源码编译安装


### 用cargo命令安装

执行下面的命令来安装xiu:

    cargo install xiu
    
执行下面的命令来查看帮助信息:

    xiu -h
 
    A secure and easy to use live media server, hope you love it!!!

    Usage: xiu [OPTIONS] 

    Options:
      -c, --config <path>   Specify the xiu server configuration file path.
      -r, --rtmp <port>     Specify the RTMP listening port(e.g.:1935).
      -t, --rtsp <port>     Specify the rtsp listening port.(e.g.:554).
      -w, --webrtc <port>   Specify the whip/whep listening port.(e.g.:8900).
      -f, --httpflv <port>  Specify the HTTP-FLV listening port(e.g.:8080).
      -s, --hls <port>      Specify the HLS listening port(e.g.:8081).
      -l, --log <level>     Specify the log level. [possible values: trace, debug, info, warn, error, debug]
      -h, --help            Print help
      -V, --version         Print version
    
### 源码编译安装

#### 克隆 Xiu

    git clone https://github.com/harlanc/xiu.git
 Checkout最新发布的版本代码：
 
    git checkout tags/<tag_name> -b <branch_name>
    
#### 编译
为了编译方便，把cargo相关的编译命令封装到了makefle中，使用下面的命令进行编译：

- 使用make local编译本地代码：

        make local && make build
- 使用make online拉取线上crates仓库代码进行编译
                
        make online && make build


#### 运行

    cd ./xiu/target/release or ./xiu/target/debug
    ./xiu -h
    
## CLI

#### 说明

可以使用配置文件或者在命令行对服务进行配置。比如：

##### 通过配置文件进行配置

    xiu -c configuration_file_path

##### 通过命令行

    xiu -r 1935 -t 5544 -f 8080 -s 8081 -l info


#### 配置文件说明

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

##### RTSP
    [rtsp]
    enabled = false
    port = 5544
    
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
    # need record the live stream or not
    need_record = true

##### Log

    [log]
    level = "info"
    [log.file]
    # 打开或者关闭输出日志到文件（注意：输出日志到控制台和文件只能2选1）.
    enabled = true
    # set the rotate
    rotate = "hour" #[day,hour,minute]
    # set the path where the logs are saved
    path = "./logs"

### 一些配置的例子

有一些现成的配置文件放在下面的目录：

    xiu/application/xiu/src/config

包括4个配置文件：

    config_rtmp.toml //只打开rtmp
    config_rtmp_hls.toml //打开 rtmp 和 hls
    config_rtmp_httpflv.toml //打开 rtmp 和 httpflv
    config_rtmp_httpflv_hls.toml //打开所有的 3 个协议
    

    
## 应用场景

##### 推流

###### RTMP推流

可以用任何推流软件或者命令工具来推RTMP流，比如使用OBS或者用ffmpeg命令行：

    ffmpeg -re -stream_loop -1 -i test.mp4 -c:a copy -c:v copy -f flv -flvflags no_duration_filesize rtmp://127.0.0.1:1935/live/test

###### RTSP推流

-  基于TCP推流(Interleaved mode)

        ffmpeg -re -stream_loop -1  -i test.mp4 -c:v copy  -c:a copy  -rtsp_transport tcp   -f rtsp rtsp://127.0.0.1:5544/live/test
    
- 基于UDP推流

        ffmpeg -re -stream_loop -1  -i test.mp4 -c:v copy  -c:a copy     -f rtsp rtsp://127.0.0.1:5544/live/test

###### 使用Whip协议推送RTC流

OBS（3.0或者更高版本）支持whip协议，按照如下配置推流：

![](https://github-production-user-asset-6210df.s3.amazonaws.com/10411078/271836332-39238b1a-d6e0-4059-bbf3-02ee298df8e7.png)

##### 播放

使用ffplay来播放 rtmp/rtsp/httpflv/hls协议的直播流:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtsp://127.0.0.1:5544/live/test
    ffplay -rtsp_transport tcp -i rtsp://127.0.0.1:5544/live/test
    ffplay -i http://localhost:8081/live/test.flv
    ffplay -i http://localhost:8080/live/test/test.m3u8

- 如何播放RTC流（使用Whep协议）

  1. 把xiu/protocol/webrtc/src/clients/目录下的文件拷贝到xiu可执行文件同级目录下；
  2. 在浏览器中打开地址：http://localhost:8900；
  3. 输入和推流地址相对应的app name和stream name；
  4. 点击Start WHEP进行播放.

![image](https://github.com/harlanc/xiu/assets/10411078/a6e1317f-0ad0-4f98-8b79-5ed8c96741f7)    

##### 转发 - 静态转推

应用场景为边缘节点的直播流被转推到源站，配置如下：

边缘节点的配置文件config_push.toml:

    [rtmp]
    enabled = true
    port = 1935
    [[rtmp.push]]
    enabled = true
    address = "localhost"
    port = 1936
    
源站节点的配置文件config.toml:

    [rtmp]
    enabled = true
    port = 1936

启动两个服务:

    ./xiu config.toml
    ./xiu config_push.toml

将一路RTMP直播流推送到边缘节点，此直播流会被自动转推到源站，可以同时播放源站或者边缘节点的直播流：

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test


    
##### 转发 - 静态回源

应用场景为播放过程中用户从边缘节点拉流，边缘节点无此流，则回源拉流，配置文件如下：

源站节点的配置文件为 config.toml:

    [rtmp]
    enabled = true
    port = 1935

 
边缘节点的配置文件为 config_pull.toml:

    [rtmp]
    enabled = true
    port = 1936
    [rtmp.pull]
    enabled = false
    address = "localhost"
    port = 1935

运行两个服务:

    ./xiu config.toml
    ./xiu config_pull.toml
    
直接将直播流推送到源站，到边缘节点请求此路直播流，边缘节点会回源拉流，可以同时播放边缘和源站节点上的直播流：

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i rtmp://localhost:1936/live/test
    
## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=harlanc/xiu&type=Date)](https://star-history.com/#harlanc/xiu)


## 鸣谢

 - [media_server](https://github.com/ireader/media-server.git)

## 其它

有任何问题请在issues提问，欢迎star和提pull request。你的关注可以让此项目走的更快更远。
 
