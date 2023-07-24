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
ARG BUILDER_TIMEZONE="Europe/Brussels"
ARG APP_VERSION="v0.6.1"
WORKDIR "/build"

# Get toolchain
RUN apk cache sync `
    && apk --update-cache upgrade --no-cache `
    && apk add --no-cache `
        "openssl-dev" "pkgconf" "git" "rustup" `
        "alpine-conf" `
    && rm -rf "/var/cache/apk" "/etc/apk/cache";
RUN rustup-init -q -y --component "cargo";

# Set TZ
RUN setup-timezone -i ${BUILDER_TIMEZONE}
ENV TZ=${BUILDER_TIMEZONE}

# Copying source and building
RUN git clone "https://github.com/harlanc/xiu.git" --branch "master" `
    && cd "xiu" `
    && git checkout -b "publish" "tags/"${APP_VERSION};
RUN cargo build --manifest-path "xiu/application/xiu/Cargo.toml" `
                --target x86_64-unknown-linux-musl `
                --release;

# 2. Run app
FROM --platform=${GLOB_PLATFORM} alpine:${RUN_VERSION} AS test_runner

# Runner args and CWD
# ARG TIMEZONE
ARG USER="appuser"
WORKDIR "/app"

# Install deps and create app user
# RUN --mount=type="cache",from="builder",src="/sys_setup/alpine_setup_answers.conf",dst="/sys_setup/" `
# apk add --no-cache "libgcc" `
RUN apk cache sync `
    && apk --update-cache upgrade --no-cache `
    && apk add "alpine-conf" `
    && setup-timezone -i ${TIMEZONE} `
    && apk del "alpine-conf" `
    && rm -rf "/var/cache/apk" "/etc/apk/cache" `
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
# ENV TZ=${TIMEZONE}
ENV SYSROOT="/dummy"
ENV PATH=${PATH}:"/app"

# Switch user, setup ports
# USER ${USER}
EXPOSE "80"
EXPOSE "80/udp"
EXPOSE "443"
EXPOSE "1935"
EXPOSE "1935/udp"
EXPOSE "8000"
EXPOSE "8000/udp"

# Start app in exec mode
ENTRYPOINT [ "xiu" ]
