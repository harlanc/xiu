# syntax=docker/dockerfile:1
# escape=\

# XIU stream/restream server
# Test image

# ---

# Glob build args
ARG BASE_VERSION="latest"
ARG RUN_VERSION="latest"
ARG PLATFORM="linux/amd64"

# Glob config args
# Apk
ARG APK_CACHE="/var/cache/apk/"

# User profile manipulations
ARG UID=10001
ARG USERNAME="appuser"

# ---

# 1. Build app
FROM --platform=${PLATFORM} alpine:${BASE_VERSION} AS builder

# - Builder args
# Deps
ARG BUILD_DEPS="openssl-dev"
ARG TOOLCHAIN="pkgconf git rust cargo"

# App source
ARG SRC_URL="https://github.com/harlanc/xiu.git"
ARG SRC_BRANCH="master"

# Directory/file settings
ARG BUILD_DIR="build"
ARG OUT_DIR="app"
ARG MANIFEST="xiu/application/xiu/Cargo.toml"
ARG BUILDOUT="xiu/target/release/xiu"

# Build args
ARG BUILD_ARCH="x86_64"
ARG BUILD_FAMILY="unix"
ARG BUILD_OS="linux"
ARG BUILD_ENV="gnu"

# - Set workdir
WORKDIR ${BUILD_DIR}

# - Get toolchain
RUN apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache ${BUILD_DEPS} ${TOOLCHAIN}; \
    apk cache clean; \
    rm -rf ${APK_CACHE};

# - Copying source and building
RUN git clone ${SRC_URL} --branch ${SRC_BRANCH};
RUN cargo build \
                --quiet \
                --manifest-path ${MANIFEST} \
                --target_arch ${BUILD_ARCH} \
                --out-dir ${OUT_DIR} \
                --release;

# ---

# 2. Run app
FROM --platform=${PLATFORM} alpine:${RUN_VERSION} AS runner

# - Runner args
# Run deps
ARG RUN_DEPS="libgcc"

# Dirs
ARG SOURCE_DIR="/build/app/"
ARG APP_DIR="/app/"
ARG SHARED_DIR="/source/"

# User creation
ARG HOME="/nonexistent"
ARG SHELL="/sbin/nologin"
ARG GECOS="Specified user"

# Port/proto aliases
ARG RTMP="1935"
ARG XIU_HTTP="8000"

# - Set workdir
WORKDIR ${APP_DIR}

# - Install deps and create app user
RUN --mount=type="cache",from="builder",src=${SOURCE_DIR},dst=${SHARED_DIR} \
    apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache ${RUN_DEPS}; \
    apk cache clean; \
    rm -rf ${APK_CACHE}; \
    cp -RT -- ${SOURCE_DIR} ${APP_DIR}; \
    adduser \
    --gecos ${GECOS} \
    --shell ${SHELL} \
    --home ${HOME} \
    --no-create-home \
    --disabled-password \
    --uid ${UID} \
    ${USERNAME};

# - Switching user
USER ${USERNAME}

# - Exposing all interesting ports
EXPOSE ${RTMP}
EXPOSE ${XIU_HTTP}

# - Launch
ENTRYPOINT [ "xiu" ]

# ---

# 3. Solo-mode
FROM runner AS solo

# - Solo-launch args
# Configs external path
ARG CONFIG_PATH="config/"

# Config mount
ARG CONFIG_MOUNT="/app/config/"

# - Adding config volume
RUN --mount=type="bind",src=${CONFIG_PATH},dst=${CONFIG_MOUNT}

# - Running with default config
CMD [ "-c", "/app/config/config_rtmp.toml" ]
