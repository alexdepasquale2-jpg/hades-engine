#!/usr/bin/env bash
# Optional: run manually or from CI after GH_TOKEN has Contents write on hades-engine.
set -euo pipefail
cd /workspace
exec ./scripts/push-to-github.sh
