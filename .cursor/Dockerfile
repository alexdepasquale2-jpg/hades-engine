FROM rust:1-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends git pkg-config libssl-dev ca-certificates \
  && rm -rf /var/lib/apt/lists/*

USER ubuntu
