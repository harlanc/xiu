# syntax=docker/dockerfile:1

# XIU restreamer
# Test image

# Creating build image
FROM alpine:latest AS builder
WORKDIR /build

# Getting git, rust and cargo
RUN apk update && apk add libgcc libssl3 openssl-dev pkgconf git rust cargo

# Copying source and building
RUN git clone https://github.com/harlanc/xiu.git --branch "master";
RUN cargo build --manifest-path "xiu/application/xiu/Cargo.toml" --release;
RUN mkdir "app" "app/config" \
    && mv "xiu/target/release/xiu" "app" \
    && cp "xiu/application/xiu/src/config/config_rtmp.toml" "app/config";

# Creating refined image
FROM alpine:latest
WORKDIR /app
ENV PATH="${PATH}:/app"

# Adding non-priv user
ARG UID=10001
RUN apk add libgcc \
    && adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser;

# Copying app
COPY --from=builder "/build/app" "/app"

# Exposing all interesting ports
EXPOSE 1935
EXPOSE 1936
EXPOSE 8080
EXPOSE 8081

# Launch
ENTRYPOINT [ "xiu" ]
CMD ["-c", "config/config_rtmp.toml"]
