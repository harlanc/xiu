# syntax=docker/dockerfile:1
# escape=`

# XIU stream/restream server
# Test image

# 1. Base image
ARG BASE_TAG="latest"
FROM alpine:${BASE_TAG} AS base

# Base deps
ARG BASE_DEPS="libgcc libssl3"

# Install deps
RUN apk update && apk upgrade && apk add ${BASE_DEPS}

# 2. Build image
FROM base as builder

# Set builder args
# Build deps
ARG BUILD_DEPS="openssl-dev"
ARG TOOLCHAIN="pkgconf git rust cargo"

# App source
ARG SRC_URL="https://github.com/harlanc/xiu.git"
ARG SRC_BRANCH="master"

# Directory/file settings
ARG BUILD_DIR="build"
ARG TARGET_DIR="app"
ARG TARGET_CONF_DIR="app/config"
ARG MANIFEST="xiu/application/xiu/Cargo.toml"
ARG COMPILED_APP="xiu/target/release/xiu"
ARG DEFAULT_CONFIG="xiu/application/xiu/src/config/config_rtmp.toml"

# Set workdir
WORKDIR ${BUILD_DIR}

# Get 'git', 'rust', 'cargo' and 'openssl-dev'
RUN apk add -y ${BUILD_DEPS} ${TOOLCHAIN}

# Copying source and building
RUN git clone ${SRC_URL} --branch ${SRC_BRANCH};
RUN cargo build --manifest-path ${MANIFEST} --release;
RUN mkdir ${TARGET_DIR} ${TARGET_CONF_DIR} `
    && mv ${COMPILED_APP} ${TARGET_DIR} `
    && cp ${DEFAULT_CONFIG} ${TARGET_CONF_DIR};

# 3. Runner
FROM base AS runner

# Runner build args
# User creation
ARG UID=10001
ARG USERNAME="appuser"
ARG OPT_HOME="/nonexistent"
ARG OPT_SHELL="/sbin/nologin"
ARG OPT_GECOS="Specified user"

# Dirs
ARG BUILDER_APP_DIR="/build/app"
ARG APP_DIR="/app"

# Port/proto aliases
ARG RTMP="1935"
ARG XIU_HTTP="8000"

# Set workdir
WORKDIR ${APP_DIR}


# Adding non-priv user
RUN apk add ${DEPS} `
    && adduser `
    --gecos ${OPT_GECOS} `
    --shell ${OPT_SHELL} `
    --home ${OPT_HOME} `
    --no-create-home `
    --disabled-password `
    --uid ${UID} `
    ${USERNAME};

# Copying app
COPY --from=base ${BUILDER_APP_DIR} ${APP_DIR}

# Setting runtime env
ENV PATH=${PATH}:${APP_DIR}

# Exposing all interesting ports
EXPOSE ${RTMP}
EXPOSE ${XIU_HTTP}

# Launch
ENTRYPOINT [ "xiu" ]
CMD [ "-c", "config/config_rtmp.toml" ]
