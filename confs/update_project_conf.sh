#!/bin/bash
if [ $# -ne 1 ]; then
    echo "USAGE: $0 <online|local>"
    echo " e.g.: $0 online"
    exit 1
fi
MODE=$1

copy_conf_files() {

    cp ./$MODE/common.Cargo.toml "../library/common/Cargo.toml"
    cp ./$MODE/h264.Cargo.toml "../library/codec/h264/Cargo.toml"
    cp ./$MODE/mpegts.Cargo.toml "../library/container/mpegts/Cargo.toml"
    cp ./$MODE/flv.Cargo.toml "../library/container/flv/Cargo.toml"
    cp ./$MODE/streamhub.Cargo.toml "../library/streamhub/Cargo.toml"
    cp ./$MODE/hls.Cargo.toml "../protocol/hls/Cargo.toml"
    cp ./$MODE/httpflv.Cargo.toml "../protocol/httpflv/Cargo.toml"
    cp ./$MODE/rtmp.Cargo.toml "../protocol/rtmp/Cargo.toml"
    cp ./$MODE/rtsp.Cargo.toml "../protocol/rtsp/Cargo.toml"
    cp ./$MODE/webrtc.Cargo.toml "../protocol/webrtc/Cargo.toml"
    cp ./$MODE/pprtmp.Cargo.toml "../application/xiu/Cargo.toml"
    cp ./$MODE/xiu.Cargo.toml "../application/xiu/Cargo.toml"
}

# do some operations
if [ "$MODE" = "online" ]; then
    echo "copy online cargo project files..."
    copy_conf_files
elif [ "$MODE" = "local" ]; then
    echo "copy local cargo project files..."
    copy_conf_files
else
    echo "not supported mode: $MODE, input <online|local>"
fi