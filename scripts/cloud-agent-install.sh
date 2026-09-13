#!/usr/bin/env bash
# Idempotent Cloud Agent bootstrap for tbc-engine.
set -euo pipefail
cd /workspace

if ! command -v cargo >/dev/null; then
  echo "cargo missing; install Rust in the environment Dockerfile." >&2
  exit 1
fi

cargo fetch
cargo test -p tbc-engine -q
cargo build -p tbc-engine --release -q

cargo fetch
cargo test -p tbc-engine -q
cargo build -p tbc-engine --release -q
