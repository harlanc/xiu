# syntax=docker/dockerfile:1
# escape=\

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
RUN apk cache sync \
    && apk --update-cache upgrade \
    && apk add --no-cache "openssl-dev" "pkgconf" "git" "rust" "cargo" \
    && apk cache clean \
    && rm -rf "/var/cache/apk";

# Copying source and building
RUN git clone "https://github.com/harlanc/xiu.git" --branch "master" \
    && cd "xiu" \
    && git checkout -b "publish" "tags/"${APP_VERSION};
RUN cargo build --manifest-path "xiu/application/xiu/Cargo.toml" \
                --release;
CMD ["echo", "Builded."]

# 2. Run app
FROM --platform=${GLOB_PLATFORM} alpine:${RUN_VERSION} AS test_runner

# Runner env
ENV SYSROOT="/dummy"
ENV PATH=${PATH}:"/app"


# Runner args and CWD
ARG USER="appuser"
WORKDIR "/app"

# Install deps and create app user
RUN apk cache sync \
    && apk --update-cache upgrade \
    && apk add --no-cache "libgcc" \
    && apk cache clean \
    && rm -rf "/var/cache/apk" \
    && adduser \
    --uid "10001" \
    --gecos "Special no-login user for pub app." \
    --shell "/sbin/nologin" \
    --home "/nonexistent" \
    --no-create-home \
    --disabled-password \
    ${USER};

# Copy app
COPY --from=builder "/build/xiu/target/release/xiu" "."

# Switch user, setup and launch
USER ${USER}
EXPOSE "1935"
ENTRYPOINT [ "xiu" ]
