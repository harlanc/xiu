# syntax=docker/dockerfile:1
# escape=`

# XIU stream/restream server
# Test image

# Glob build args, config, and user management 
ARG BASE_VERSION="latest"
ARG RUN_VERSION="latest"
ARG GLOB_PLATFORM="linux/amd64"

# 1. Build app
FROM --platform=${GLOB_PLATFORM} alpine:${BASE_VERSION} AS builder

# Builder args and CWD
ARG APP_VERSION="v0.6.1"
WORKDIR "/build"

# Get toolchain
RUN apk cache sync `
    && apk --update-cache upgrade `
    && apk add --no-cache `
        "openssl-dev" "pkgconf" "git" "rustup" `
        "alpine-conf" `
    && apk cache clean `
    && rm -rf "/var/cache/apk";
COPY "./alpine_setup_answers.conf" "/sys_setup/"
RUN rustup-init -y;
RUN setup-alpine -f "/sys_setup/alpine_setup_answers.conf"

# RUN setup-timezone -p ${TZ} && setup-ntp "busybox"
# RUN rustup component add rust-std-x86_64-unknown-linux-musl

# Copying source and building
RUN git clone "https://github.com/harlanc/xiu.git" --branch "master" `
    && cd "xiu" `
    && git checkout -b "publish" "tags/"${APP_VERSION};
RUN cargo build --manifest-path "xiu/application/xiu/Cargo.toml" `
                --target x86_64-unknown-linux-musl;
                # --release;

# 2. Run app
FROM --platform=${GLOB_PLATFORM} alpine:${RUN_VERSION} AS test_runner

# Runner args and CWD
ARG USER="appuser"
WORKDIR "/app"

# apk add --no-cache "libgcc"
# Install deps and create app user
RUN apk cache sync `
    && apk --update-cache upgrade `
    # && apk add "alpine-conf" `
    && apk cache clean `
    && rm -rf "/var/cache/apk" `
    && adduser `
    --uid "10001" `
    --gecos "Special no-login user for app." `
    --shell "/sbin/nologin" `
    --home "/nonexistent" `
    --no-create-home `
    --disabled-password `
    ${USER};

# Copy app
COPY --from=builder "/build/xiu/target/release/xiu" "."

# Runner env
ENV SYSROOT="/dummy"
ENV PATH=${PATH}":/app"

# Switch user, setup ports
# USER ${USER}
EXPOSE "80"
EXPOSE "80/udp"
EXPOSE "443"
EXPOSE "443/udp"
EXPOSE "1935"
EXPOSE "1935/udp"
EXPOSE "8000"
EXPOSE "8000/udp"

# Start app in exec mode
ENTRYPOINT [ "xiu" ]
