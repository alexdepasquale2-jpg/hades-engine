#!/usr/bin/env bash
# Generate self-signed TLS material for local/docker QUIC gateway.
set -euo pipefail
DIR="${1:-$(dirname "$0")/tls}"
mkdir -p "$DIR"
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$DIR/key.pem" \
  -out "$DIR/cert.pem" \
  -days 365 \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
echo "Written $DIR/cert.pem and $DIR/key.pem"
