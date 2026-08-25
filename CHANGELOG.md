# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-08-25

First public release of the MBT-native MMORPG engine vertical slice (M1–M18).

### Added

- **tbc-engine** — AUM core, dual PMR shards, NPMR Academy/Dream frames, entropy ledger, netcode rewind, RWW bus (memory + NATS), shard handoff, reincarnation planner, ruleset-driven verbs, psi/social/consent, assist, wire transport, and ops snapshots.
- **tbc-sdk** — Typed HTTP client (`TbcHttpClient`) and re-exports of wire protocol builders.
- **tbc-server** — Debug HTTP/WebSocket server on port 6014 with web UI.
- **tbc-gateway** — QUIC TLS gateway (4433) and ops HTTP (9443).
- Rulesets: `pmr.v1`, `npmr.academy.v1`, `npmr.dream.v1`.
- Browser client: `web/sdk/tbc-client.js`.
- CI workflow (format, build, test, clippy).
- Docker Compose stack and ops runbook.

### Publishing

Crates published to [crates.io](https://crates.io): `tbc-engine`, `tbc-sdk`.

Tag `v0.1.0` on `main` triggers the release workflow when `CARGO_REGISTRY_TOKEN` is configured.

[0.1.0]: https://github.com/unfaithful/hades-engine/releases/tag/v0.1.0
