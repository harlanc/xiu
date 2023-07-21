# syntax=docker/dockerfile:1
# escape=\

# XIU stream/restream server
# Test image

# Glob build args, config, and user management 
ARG BASE_VERSION="latest"
ARG RUN_VERSION="latest"
ARG PLATFORM="linux/amd64"
ARG APK_CACHE="/var/cache/apk/"

# 1. Build app
FROM --platform=${PLATFORM} alpine:${BASE_VERSION} AS builder

# Builder args - source settings, directories and CWD
ARG XIU_VERSION="v0.6.1"
ARG BUILD_DIR="/build/"
WORKDIR ${BUILD_DIR}

# Get toolchain
RUN apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache "openssl-dev" "pkgconf" "git" "rust" "cargo"; \
    apk cache clean; \
    rm -rf ${APK_CACHE};

# Copying source and building
RUN git clone "https://github.com/harlanc/xiu.git" --branch "master" \
    && cd "xiu" \
    && git checkout -b "publish" "tags/"${XIU_VERSION} \
    && cd ${BUILD_DIR};
RUN cargo build --manifest-path "xiu/application/xiu/Cargo.toml" \
                --release;
RUN echo "Builded."

# 2. Run app
FROM --platform=${PLATFORM} alpine:${RUN_VERSION} AS test_runner

# Runner args - dirs, user - and CWD
WORKDIR "/app"

# Install deps and create app user
RUN --mount=type=bind,from=builder,src=/build/xiu/target/release,dst=/mnt/source \
    apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache "libgcc" \
    apk cache clean; \
    rm -rf ${APK_CACHE}; \
    cp -RT -- "/source/*" "/app"; \
    adduser \
    --uid "10001" \
    --gecos "Special no-login user for pub app." \
    --shell "/sbin/nologin" \
    --home "/nonexistent" \
    --no-create-home \
    --disabled-password \
    "appuser";

# Switch user, setup and launch
USER "appuser"
EXPOSE "1935"
EXPOSE "8000"
ENTRYPOINT [ "sh" ]
