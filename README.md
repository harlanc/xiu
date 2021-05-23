# Xiu
**Xiu is a live server written by Rust.**


## Functionalities

- [x] rtmp
  - [x] publish and play (now only simple handshake is supported.)
  - [x] relay: static push
  - [x] relay: pull
- [ ] httpflv
- [ ] hls
- ...

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

    cd ./xiu/application
    
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



## Change Logs

[2021-05-15]

- Impl : Coding for pull is finished.


[2021-05-02]

- Impl : Coding for static push is finished.


[2021-04-16]

- Fix:  obs publish-> unpublish->publish ,,  ffplay cannot play successfully

[2021-04-15]

- Improve: remove build warnings.

[2021-04-14]

- Fix bug: when shutdown ffplayer ,the OBS publisher will be reconnected automatically.


[2021-04-11]

- Impl: add flush\_timeout and read\_timeout functinos.
- Impl: add some log print configuratinos.
- Fix bug: Chunk header with the save csid is not saved.
- Impl: add unsubscribe and unpublish logic in server\_session and channels models.

[2021-04-10]

- Improve: change use libraries format.

[2021-04-09]

- Update README.

[2021-04-08]

- Fix: replace oneshot by mpsc channel,may be improved.

## Star History

[link](https://star-history.t9t.io/#harlanc/xiu)
