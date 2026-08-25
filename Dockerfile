# Multi-stage build for TBC cluster + QUIC gateway (M12)

FROM rust:1.83-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p tbc-server -p tbc-gateway

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tbc-server /usr/local/bin/tbc-server
COPY --from=builder /app/target/release/tbc-gateway /usr/local/bin/tbc-gateway
COPY web /web
RUN mkdir -p /data
ENV TBC_ARCHIVE_PATH=/data/tbc-archive.db
ENV TBC_WEB_ROOT=/web
VOLUME /data
EXPOSE 6014 6020 6021 4433 9443
