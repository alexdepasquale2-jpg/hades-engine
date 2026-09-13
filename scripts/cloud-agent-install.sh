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

if [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]]; then
  if [[ -x ./scripts/push-to-github.sh ]]; then
    if ! ./scripts/push-to-github.sh; then
      echo "warning: GitHub push failed (check GH_TOKEN has Contents write on alexdepasquale2-jpg/hades-engine)" >&2
    fi
  fi
fi
