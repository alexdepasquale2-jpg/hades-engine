#!/usr/bin/env bash
# Push this repo to GitHub (default: alexdepasquale2-jpg/hades-engine).
set -euo pipefail

OWNER="${GITHUB_OWNER:-alexdepasquale2-jpg}"
REPO="${GITHUB_REPO:-hades-engine}"
BRANCH="${GITHUB_BRANCH:-main}"
REMOTE="${GITHUB_REMOTE:-github}"

if [[ -z "${GH_TOKEN:-}" && -z "${GITHUB_TOKEN:-}" ]]; then
  echo "Set GH_TOKEN (or GITHUB_TOKEN) with repo scope, then re-run." >&2
  echo "  https://github.com/settings/tokens" >&2
  exit 1
fi

TOKEN="${GH_TOKEN:-${GITHUB_TOKEN}}"

if ! git remote get-url "$REMOTE" &>/dev/null; then
  git remote add "$REMOTE" "https://github.com/${OWNER}/${REPO}.git"
fi

git push "https://x-access-token:${TOKEN}@github.com/${OWNER}/${REPO}.git" "${BRANCH}:${BRANCH}"
echo "Pushed ${BRANCH} -> https://github.com/${OWNER}/${REPO}"
