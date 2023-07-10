# syntax=docker/dockerfile:1

# XIU restreamer
# Test image

# Creating build image
ARG BUILDER_TAG="latest"
FROM alpine:${BUILDER_TAG} AS builder

# Define some handy args
ARG DEPS="libgcc libssl3 openssl-dev"
ARG TOOLCHAIN="pkgconf git rust cargo"
ARG CWD="/build"
ARG SOURCE_URL="https://github.com/harlanc/xiu.git"
ARG MANIFEST="xiu/application/xiu/Cargo.toml"
ARG COMPILED_APP="xiu/target/release/xiu"
ARG DEFAULT_CONFIG="xiu/application/xiu/src/config/config_rtmp.toml"

# Set workdir
WORKDIR ${CWD}

# Getting git, rust and cargo
RUN apk update && apk add ${DEPS} ${TOOLCHAIN}

# Copying source and building
RUN git clone ${SOURCE_URL} --branch "master";
RUN cargo build --manifest-path ${MANIFEST} --release;
RUN mkdir "app" "app/config" \
    && mv ${COMPILED_APP} "app" \
    && cp ${DEFAULT_CONFIG} "app/config";

# Creating refined image
FROM alpine:latest

# Runtime args
ARG DEPS="libgcc"
ARG UID=10001
ARG USERNAME="appuser"
ARG CWD="/app"
ARG DEFAULT_CONFIG="config/config_rtmp.toml"
ARG RTMP="1935"
ARG RTMP_PUSH="1936"
ARG HLS="8080"
ARG HTTPFLV="8081"

# Set workdir
WORKDIR ${CWD}

# Adding non-priv user
RUN apk add ${DEPS} \
    && adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid ${UID} \
    ${USERNAME};

# Copying app
COPY --from=builder "/build/app" "/app"

# Exposing all interesting ports
EXPOSE ${RTMP}
EXPOSE ${RTMP_PUSH}
EXPOSE ${HLS}
EXPOSE ${HTTPFLV}

# Launch
ENTRYPOINT [ "xiu" ]
CMD ["-c", ${CONFIG}]
