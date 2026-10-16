# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.14.0] - 2026-10-17
- fix release error
- fix: update crate CHANGELOG and release error
- fix: split config crates library
- fix crates check
- remove rust.yml
- fix: use cargo registry for release publish checks
- update checkout to v7
- update the version number of the crates.io dependency library.
- fix: version check error
- fix: isolate crates during release publishing
- chore: automate release workflow
- Pr/s3 (#187)
- remove deprecated calls (#186)
- hls prefix support (#185)
- log ports from listeners' local addrs (#189)
- fix GOP cache memory leak for audio-only streams (#191)
- add STRICT_ARRAY support (#183)
- initial json support (#182)
- Pr/versions (#184)
- use config object instead of single val (#181)
- kick_stream (#180)
- fix on_hls message (#179)
- update
- add sponsor recall.ai
- add recall logo
- Merge pull request #177 from nikitos/hls_params3
- Update protocol/hls/src/m3u8.rs
- Merge branch 'orig' into hls_params3
- Merge pull request #176 from nikitos/hls_params2
- Merge branch 'hls_params2' into hls_params3
- fix spaces
- Merge branch 'hls_params2' into hls_params3
- fix version
- Merge branch 'orig' into hls_params2
- Merge pull request #175 from nikitos/hls_notify
- Merge branch 'orig' into hls_params2
- Merge branch 'orig' into hls_notify
- Update protocol/hls/src/flv2hls.rs
- Merge pull request #174 from nikitos/hls_params
- Update protocol/hls/src/remuxer.rs
- Update application/xiu/README.md
- use hls notification
- on_hls notification
- use one config object
- move config to separate module
- add examples
- fix tests
- remove unused
- add some hls params
- Merge pull request #172 from feiyangyy/master
- fix(rtp_aac): fix rtp seq_number overflow of the audio stream
- fix(rtp_h264): solve rtsp adding overflow issue
- feat(application): add rtsp example config
- Merge pull request #164 from zamazan4ik/patch-1
- feat: enable LTO and codegen-units = 1 optimization
- Merge pull request #160 from huiminghao/rtsp_udp_fix
- ignore lint warning:non_local_definitions
- rtsp udp remote address fix
- Merge pull request #156 from akadan47/fix-hls-compatibility-security
- Add content types for HLS playlist & segments. Add HLS path parsing for security (countering path traversal attacks).
- Merge pull request #155 from Morb0/fix
- Fix local config dependencies
- Merge pull request #152 from vaptu/master
- feat: add rtmp client session timeout option


## [0.13.0] - 2021-08-11
- Feat: Abstract streamhub message notifications.  by @karaler 
- Feat: Add optional push_password for simple authentication. by @Ceron257 
- Feat: Implement Authorization header support for WebRTC. by @Ceron257 
- Fix: XIU cannot receive RTSP stream published by gstreamer #135
- Feat: Support Rtsp pull client logic, and you can pull a remote RTSP stream using HTTP API to XIU now.

## [0.12.7] - 2021-05-18
- Fix: RTMP publish single AAC from ffmpeg client.  by @suzp1984
- Fix: RTMP Auth failing due to empty string query string in packet. by @radiohertz
- Improve: the xiu application README for new beginners. by @radiohertz
- Fix: the xiu application version.

## [0.12.6] - 2021-04-03
- Fix bug that the whip stream can not be established successfully #111.
- Fix the issue of not correctly recognizing Opus encoding parameters.
- Fix the issue of not being able to read HTTP resources when pulling streams using the WHEP.

## [0.12.5]
- Support querying more detailed statistic data by adding two new HTTP APIs.
- Fix publishing RTSP stream error caused by network problem. by @bailb
- Fix the bug that stopping the playback of RTSP stream leads to push(publish) failure.
- Upgrade failure library.

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



