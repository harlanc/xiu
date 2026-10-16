# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.5.6] - 2026-10-17
- fix: split config crates library
- Pr/s3 (#187)
- hls prefix support (#185)
- log ports from listeners' local addrs (#189)
- Pr/versions (#184)
- Update protocol/hls/src/m3u8.rs
- Merge branch 'hls_params2' into hls_params3
- Merge branch 'orig' into hls_notify
- Update protocol/hls/src/flv2hls.rs
- Update protocol/hls/src/remuxer.rs
- use hls notification
- on_hls notification
- use one config object
- fix tests
- remove unused
- add some hls params
- ignore lint warning:non_local_definitions
- Add content types for HLS playlist & segments. Add HLS path parsing for security (countering path traversal attacks).


## [0.5.5] - 2021-08-11
- Some refactor work.

## [0.5.4] - 2021-05-18
- fix RTMP publish single AAC from ffmpeg client.

## [0.5.3] - 2021-03-15
- Upgrade failure library.
- Some changes for statistics feature.

## [0.5.2] - 2021-02-28
- Refactor: remove the dependecy of HLS on RTMP.

## [0.5.1]
- Fix RTMP build error.
  
## [0.5.0]
- Support auth.
  
## [0.4.3]
- Remove no used "\n" for error message.
- Replace hyper HTTP library with axum.
- Receive and process sub event result.

## [0.4.2]
- Reference streamhub new version v0.1.2.
  
## [0.4.0]
- Support record.
  
## [0.3.0]
- Do some refactoring.
- Reference bytesio v0.3.0.
  
## [0.2.1]
- Reference xflv new version v0.2.1.
- Reference rtmp new version v0.3.1.

## [0.2.0]
- Support stream status notify using HTTP.

## [0.1.2]
- Refactor codes
- Reference rtmp/xflv/mpegts new version.

## [0.1.1]
Change the listening port type.

## [0.1.0]
- Fix the error chain.
  
## [0.0.8]
- fix #16
- clear ts ans m3u8 files when the rtmp streasm is interupted.

## [0.0.4]
- Reference rtmp version v0.0.8

## [0.0.3]
- Reference xmpegts version v0.0.2

## [0.0.2]
- Some improvements.

## [0.0.1]
- Support remux RTMP stream to hls stream.