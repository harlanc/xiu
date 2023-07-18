# Pre-syntax block
# syntax=docker/dockerfile:1

# XIU restreamer
# Test image

# Creating build image
ARG BUILDER_TAG="latest"
FROM alpine:${BUILDER_TAG} AS builder

# Define some handy args
ARG DEPS="libgcc libssl3 openssl-dev"
ARG TOOLCHAIN="pkgconf git rust cargo"
ARG SOURCE_URL="https://github.com/harlanc/xiu.git"
ARG SRC_BRANCH="master"
ARG BUILD_DIR="build"
ARG TARGET_DIR="app"
ARG TGT_APPCONFIG_DIR="app/config"
ARG MANIFEST="xiu/application/xiu/Cargo.toml"
ARG COMPILED_APP="xiu/target/release/xiu"
ARG DEFAULT_CONFIG="xiu/application/xiu/src/config/config_rtmp.toml"

# Set workdir
WORKDIR ${BUILD_DIR}

# Getting git, rust and cargo
RUN apk update && apk add ${DEPS} ${TOOLCHAIN}

# Copying source and building
RUN git clone ${SOURCE_URL} --branch ${SRC_BRANCH};
RUN cargo build --manifest-path ${MANIFEST} --release;
RUN mkdir ${TARGET_DIR} ${TGT_APPCONFIG_DIR} \
    && mv ${COMPILED_APP} ${TARGET_DIR} \
    && cp ${DEFAULT_CONFIG} ${TGT_APPCONFIG_DIR};

# Creating refined runner
FROM alpine:latest

# Pre-run args
ARG DEPS="libgcc"
ARG UID=10001
ARG USERNAME="appuser"
ARG SHELL_HOMEDIR="/nonexistent"
ARG SHELL="/sbin/nologin"
ARG GECOS_OPT="Specified user"
ARG BUILDER_SRC_DIR="/build/app"
ARG RUNNER_APP_DIR="/app"
ARG APP="xiu"
ARG DEFAULT_CONFIG="config/config_rtmp.toml"
ARG HTTP="80"
ARG HTTP_UDP="80/udp"
ARG HTTPS="443"
ARG RTMP="1935"
ARG RTMP_PUSH="1936"
ARG HLS="8080"
ARG HTTPFLV="8081"

# Set workdir
WORKDIR ${RUNNER_APP_DIR}

# Adding non-priv user
RUN apk add ${DEPS} \
    && adduser \
    --gecos ${GECOS_OPT} \
    --shell ${SHELL} \
    --home ${SHELL_HOMEDIR} \
    --no-create-home \
    --disabled-password \
    --uid ${UID} \
    ${USERNAME};

# Copying app
COPY --from=builder ${BUILDER_SRC_DIR} ${RUNNER_APP_DIR}

# Setting runtime env
ENV PATH=${PATH}:${RUNNER_APP_DIR}
ENV APPNAME=${APP}
ENV CONFIG=${DEFAULT_CONFIG}

# Exposing all interesting ports
EXPOSE ${HTTP}
EXPOSE ${HTTP_UDP}
EXPOSE ${HTTPS}
EXPOSE ${RTMP}
EXPOSE ${RTMP_PUSH}
EXPOSE ${HLS}
EXPOSE ${HTTPFLV}

# Launch
ENTRYPOINT [ "xiu" ]
CMD [ "-c", ${CONFIG} ]
