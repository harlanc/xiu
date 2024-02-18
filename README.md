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



[English Doc](https://harlanc.github.io/) [中文文档](https://harlanc.github.io/zh-cn/) 



Xiu is a simple,high performance and secure live media server written in pure Rust, it now supports popular live protocols like RTMP[cluster]/RTSP/WebRTC[Whip/Whep]/HLS/HTTP-FLV.

## Features
- [x] Support multiple platforms(Linux/MacOS/Windows).
- [x] Support RTMP.
   - [x] Support publishing or subscribing H.264/AAC streams.
   - [x] Support GOP cache which can be configured in the configuration file.
   - [x] Support protocol conversion from RTMP to HTTP-FLV/HLS.
   - [x] Support cluster.
- [x] Support RTSP.
  - [x] Support publishing or subscribing H.265/H.264/AAC stream over both TCP(Interleaved) and UDP.
  - [x] Support protocol conversion from RTSP to RTMP/HLS/HTTP-FLV.
- [x] Support WebRTC(Whip/Whep).
  - [x] Support publishing rtc stream using Whip.
  - [x] Support subscribing rtc stream using Whep.
  - [x] Support protocol conversion from WHIP to RTMP/HLS/HTTP-FLV.
- [x] Support HTTP-FLV/HLS protocols(Transferred from RTMP/RTSP).
- [x] Support configuring the service using command line or a configuration file.
- [x] Support HTTP API/Notifications.
  - [x] Support querying stream information.
  - [x] Support notification of stream status.
- [x] Support token authentications.
- [x] Support recording live streams into HLS files(m3u8+ts).


## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=harlanc/xiu&type=Date)](https://star-history.com/#harlanc/xiu)


## Thanks

 - [media_server](https://github.com/ireader/media-server.git)

## Others

Open issues if you have any problems. Star and pull requests are welcomed. Your stars can make this project go faster and further.
 
