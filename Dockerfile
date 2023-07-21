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

# Builder args
# Deps, source settings, directories, build args
ARG BUILD_DEPS="openssl-dev"
ARG TOOLCHAIN="pkgconf git rust cargo"
ARG SRC_URL="https://github.com/harlanc/xiu.git"
ARG SRC_BRANCH="master"
ARG SRC_TAG="v0.6.1"
ARG BUILD_DIR="/build/"
ARG REPOROOT="xiu"
ARG MANIFEST="xiu/application/xiu/Cargo.toml"
ARG TARGET_TRIPLE="x86_64-unknown-linux-gnu"

# Set workdir
WORKDIR ${BUILD_DIR}

# Get toolchain
RUN apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache ${BUILD_DEPS} ${TOOLCHAIN}; \
    apk cache clean; \
    rm -rf ${APK_CACHE};

# Copying source and building
RUN git clone ${SRC_URL} --branch ${SRC_BRANCH} \
    && cd ${REPOROOT} \
    && git checkout -b "publish" "tags/"${SRC_TAG} \
    && cd ${BUILD_DIR};
RUN cargo build \
                --manifest-path ${MANIFEST} \
                --release;
RUN echo "Builded."

# 2. Run app
FROM --platform=${PLATFORM} alpine:${RUN_VERSION} AS runner

# Runner args
# Deps, dirs, user creation, port/proto aliases
ARG RUN_DEPS="libgcc"
ARG SOURCE_DIR="/build/xiu/target/release/"
ARG SHARED_DIR="/source/"
ARG INSTALL_DIR="/app"
ARG UID="10001"
ARG USERNAME="appuser"
ARG HOME="/nonexistent"
ARG SHELL="/sbin/nologin"
ARG GECOS="Specified user"
ARG RTMP="1935"
ARG XIU_HTTP="8000"

# Set workdir
WORKDIR ${INSTALL_DIR}

# Install deps and create app user
RUN --mount=type="cache",from="builder",src=${SOURCE_DIR},dst=${SHARED_DIR} \
    apk cache sync; \
    apk --update-cache upgrade; \
    apk add --no-cache ${RUN_DEPS}; \
    apk cache clean; \
    rm -rf ${APK_CACHE}; \
    cp -RT -- ${SHARED_DIR}"/*" ${INSTALL_DIR}; \
    adduser \
    --gecos ${GECOS} \
    --shell ${SHELL} \
    --home ${HOME} \
    --no-create-home \
    --disabled-password \
    --uid ${UID} \
    ${USERNAME};

# Switching user
USER ${USERNAME}

# Exposing all interesting ports
EXPOSE ${RTMP}
EXPOSE ${XIU_HTTP}

# Launch
ENTRYPOINT [ "xiu" ]
