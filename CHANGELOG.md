# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Repository trimmed to **`tbc-engine`** only (removed debug HTTP server, QUIC gateway, SDK, web UI, Docker/ops). Host binaries belong in separate repos or your game project.

### Added

- **M20** — `Verb::Assist` and `Verb::Speak` processed through netcode intent queue, tick stepping, and rewind-replay.

## [0.1.0] - 2026-08-25

First public release of the MBT-native MMORPG engine vertical slice (M1–M18).

### Added

- **tbc-engine** — AUM core, dual PMR shards, NPMR Academy/Dream frames, entropy ledger, netcode rewind, RWW bus (memory + NATS), shard handoff, reincarnation planner, ruleset-driven verbs, psi/social/consent, assist, wire transport, and ops snapshots.
- Rulesets: `pmr.v1`, `npmr.academy.v1`, `npmr.dream.v1`.
- CI workflow (format, build, test, clippy).

### Publishing

Crate published to [crates.io](https://crates.io): `tbc-engine`.

Tag `v0.1.0` on `main` triggers the release workflow when `CARGO_REGISTRY_TOKEN` is configured.

[0.1.0]: https://github.com/alexdepasquale2-jpg/hades-engine/releases/tag/v0.1.0
