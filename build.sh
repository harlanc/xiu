#!/bin/sh

sudo rm -rf dst \
    && mkdir dst \
    && docker run -v ./src:/app/src \
        -v ./Cargo.toml:/app/Cargo.toml -v ./Cargo.lock:/app/Cargo.lock \
        -v ./application:/app/application \
        -v ./confs:/app/confs \
        -v ./library:/app/library \
        -v ./protocol:/app/protocol \
        -v ./rtsp.toml:/app/rtsp.toml \
        -v ./dst:/app/target -t rtxp_server:builder /root/.cargo/bin/cargo build --bin xiu --release \
    && cp dst/release/xiu ./ \
    && sudo rm -rf dst

zip xiu.zip xiu  && scp ./xiu.zip root@10.10.181.175:/data/workspace/rtxp_server/ && ssh root@10.10.181.175