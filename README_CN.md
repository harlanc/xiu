# XIU


[![crates.io](https://img.shields.io/crates/v/xiu.svg)](https://crates.io/crates/xiu)
[![](https://app.travis-ci.com/harlanc/xiu.svg?branch=master)](https://app.travis-ci.com/github/harlanc/xiu)
[![](https://img.shields.io/discord/894502149764034560?logo=discord)](https://discord.gg/gS5wBRtpcB)
![wechat](https://img.shields.io/:微信-harlancc-blue.svg)
![qqgroup](https://img.shields.io/:QQ群-24893069-blue.svg)


Xiu是用纯rust开发的一款简单和安全的流媒体服务器，目前支持流行的流媒体协议包括RTMP/HLS/HTTPFLV（将来有可能支持其它协议），可以单点部署，也可以用relay功能来部署集群。

## 功能

- [x] RTMP
  - [x] 发布直播流和播放直播流
  - [x] 转发：静态转推和静态回源
- [x] HTTPFLV
- [x] HLS
- [ ] SRT


## 准备工作
#### 安装 Rust and Cargo


[Document](https://doc.rust-lang.org/cargo/getting-started/installation.html)

## 安装和运行

有两种方式来安装xiu：
 
 - 直接用cargo来安装
 - 源码编译安装


### 用cargo命令安装

执行下面的命令来安转xiu:

    cargo install xiu
    
执行下面的命令来启动服务:

    xiu configuration_file_path/confit.toml
    
### 源码编译安装

#### 克隆 Xiu

    git clone https://github.com/harlanc/xiu.git
 Checkout最新发布的版本代码：
 
        git checkout tags/<tag_name> -b <branch_name>
    

    
#### 编译

    cd ./xiu/application/xiu
    cargo build --release
#### 运行

    cd ./xiu/target/release
    ./xiu config.toml
    
## 配置

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

    

    
## 应用场景

##### 推流

可以用任何推流软件或者命令工具来推RTMP流，比如使用OBS或者用ffmpeg命令行：

    ffmpeg -re -stream_loop -1 -i test.mp4 -c:a copy -c:v copy -f flv -flvflags no_duration_filesize rtmp://127.0.0.1:1935/live/test110


##### 播放

使用ffplay来播放 rtmp/httpflv/hls协议的直播流:

    ffplay -i rtmp://localhost:1935/live/test
    ffplay -i http://localhost:8081/live/test.flv
    ffplay -i http://localhost:8080/live/test/test.m3u8
    
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

[link](https://star-history.t9t.io/#harlanc/xiu)

## 鸣谢

 - [media_server](https://github.com/ireader/media-server.git)

## 其它

有任何问题请在issues提问，欢迎star和提pull request。微信号：harlancc
 
