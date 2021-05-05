# Xiu
Xiu is a live server written by Rust.


## Functionalities

- [x] RTMP 
  - [x] publish and play
  - [ ] relay: static push
  - [ ] relay: static pull
- [ ] HTTPFLV
- [ ] HLS
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
    
    ./application
    
#### Push

I use OBS to push a live rtmp stream.


#### Play

I use ffplay to play rtmp live stream:

    ffplay -i rtmp://localhost:1935/live/test

## Change Logs

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

