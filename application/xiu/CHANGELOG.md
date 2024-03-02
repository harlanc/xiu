# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.12.4]
- Fix the failure in generating Docker images.

## [0.12.0]
- Support pull/push authentication #95 .
- Support publishing pre-built images and docker images using github action #100 Thanks @svenstaro .
- Fix the issue of incomplete HLS recording file generation #101 Thanks @GameEgg .
- Refactor: extract http mod from RTSP/Webrtc to common library.
- Refactor: extract amf0 mod from RTSP to XFLV library.
- Refactor: remove the dependency of HLS on RTMP.
- Refactor: remove the dependency of HTTP-FLV on RTMP.
- Refactor api_kick_off_client of streamhub to simplify the process.
- Update denpendency library of WebRTC from opus-rs to audiopus to support cross compile.
- Use reqwest's vender feature referenced in streamhub to support cross compile.

## [0.10.0]
- Remove no used "\n" for error message.
- Support remux from WHIP to RTMP.

## [0.9.1]
- Support WebRTC(whip/whep).

## [0.8.0]
- Support HLS record.

## [0.7.0]
- Support RTSP.

## [0.6.1]
- Fix error that cannot receive rtmp stream pushed from GStreamer.
- Fix rtmp cts parse error.
- Fix RTMP examples in README.

## [0.6.0]
- Support notify stream status.
- Support HTTP API to kickoff clients.
- Add a http-server for testing http notify.
- Add a pull rtmp and push rtmp example: pprtmp.
- Fix some RTMP library bugs.

## [0.5.0]
- Support rtmp gop number configuration.
- Support query stream information using HTTP api.



