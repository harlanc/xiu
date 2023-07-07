# syntax=docker/dockerfile:1

# XIU test
# Local env

# Creating build layer
ARG RUST_VERSION=latest
ARG APP_NAME=XIU
FROM rust:${RUST_VERSION} AS builder
ARG APP_NAME
WORKDIR /build

    
# Copying source and building
RUN git clone https://github.com/harlanc/xiu.git \
    && cd "xiu/application/xiu" \
    && cargo build --release \
    && mkdir "/build/app" \
    # String below can be used with "--parents" flag to avoid
    # previous string, but not recommended
    && mkdir "/build/app/config" \
    && mv "/build/xiu/target/release/xiu" "/build/app/" \
    && cp "./src/config/config_rtmp.toml" "/build/app/config/"

# Creating refined image 
FROM alpine:latest
WORKDIR /app
ENV PATH="${PATH}:/app"

# Adding non-priv user
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Copying app
COPY --from=builder "/build/app/xiu" "/app/xiu"
COPY --from=builder "/build/app/config/config_rtmp.toml" "/app/config/config_rtmp.toml"

# Launch
CMD ["xiu", "-c", "/app/config/config_rtmp.toml"]
