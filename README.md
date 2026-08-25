# The Big Computer (TBC) Engine

An MBT-native MMORPG engine implementing the architecture from **The Big Computer Engine Specification** — consciousness-first simulation where AUM_Core is authority, TBC renders PMR one Δt at a time, and the EntropyLedger is the private character sheet.

## What this repo implements

**M1–M18 vertical slice** of the spec:

| Module | Spec section | Status |
| --- | --- | --- |
| `DeltaTClock` | §5 Δt | Fixed 50 ms timestep, accumulator, max 4 catch-up |
| `EntropyLedger` | §8 | Private S scalar, delayed noisy consequences |
| `IUOC` / `FWAU` | §3 | Soul bind, quality snapshot, experience packets |
| `SoulArchive` | §3 M9 | SQLite persistence for souls + experience packets |
| `HierGrid` | §4 | 32/128/512 m spatial hash, interest queries |
| `ProbabilitySurface` | §7 | Beam B=16, D=8, A=6 intent-biased prune |
| `IslandManager` | §7 M6 | ~40 islands, observe-collapse, step profiler |
| `GuardrailState` | M8 | Rate limits, beam budget throttle, stall telemetry |
| `NetcodeState` | §7 | Snapshot ring N=8, rewind-replay, corrections |
| `ShardBounds` | §10 M7/M11 | PMR shard-00 / shard-01, overlap strip, RWW cross events |
| `Reincarnation planner` | §11 M7 | K=5 ranked offers, accept + rebirth |
| `RwwBus` | §9 M10 | In-memory + NATS JetStream (`TBC_RWW` stream) |
| `transport` | §10 | Framed JSON reliable + 36-byte move datagrams |
| `CrdtProp` / `PropStore` | M14 | Or-set props in NPMR frames from interact |
| `VerbPolicies` | M13/M15 | Attack/interact/speak from ruleset JSON |
| `query_psi` / `speak` | M15 | Ruleset-gated psi scopes + social broadcast |
| `assist` | M16 | Consent-gated heal + NPMR AI practice mode |
| `OpsSnapshot` | M12 | `/health`, `/ready`, Prometheus `/metrics` |
| `Frame` PMR + NPMR | §4–6 | Dual PMR shards + NPMR Academy |
| Debug server | §10 | HTTP + WebSocket — cluster **6014** |
| `tbc-sdk` | M17 | Typed HTTP client + wire protocol re-exports |
| QUIC gateway | §10 M12 | TLS/mTLS QUIC **4433**, ops HTTP **9443** |

## Quick start

```bash
cargo run -p tbc-server --release
```

Open **http://127.0.0.1:6014**

Default soul archive: `data/tbc-archive.db` (override with `TBC_ARCHIVE_PATH`).

Optional NATS JetStream RWW:

```bash
docker run --rm -p 4222:4222 nats:2.10 -js
export TBC_NATS_URL=nats://127.0.0.1:4222
cargo run -p tbc-server --release
```

Without `TBC_NATS_URL`, RWW stays in-memory (dev mode).

### Multi-node PMR (M11)

Run two shard processes sharing the same archive and NATS RWW:

```bash
docker run --rm -p 4222:4222 nats:2.10 -js
export TBC_NATS_URL=nats://127.0.0.1:4222
export TBC_ARCHIVE_PATH=data/tbc-archive.db

TBC_SHARD_ID=0 cargo run -p tbc-server --release   # http://127.0.0.1:6020
TBC_SHARD_ID=1 cargo run -p tbc-server --release   # http://127.0.0.1:6021
```

Walk east on shard-00 past the seam; the soul unbinds locally and rebinds on shard-01 via `rww.shard.cross`. Override port with `TBC_PORT`.

Default cluster mode (no `TBC_SHARD_ID`) keeps in-process handoff on port **6014**.

1. **Partition FWAU** — bind a new IUOC avatar on PMR shard-00 (west)
2. **Resume soul** — after restart, resume the same IUOC from the archive (stored in browser localStorage)
3. **WASD** — move; walk **east (→)** past the yellow seam at x=0 for shard handoff
4. **Unbind / death** — experience packets persist; reincarnation offers use archived history
5. **Enter NPMR-Academy** — frame handoff via RWW
6. **B** — blink in NPMR · **F** strike · **E** interact · **G** assist (NPMR AI practice) · **T** speak · psi scopes

### QUIC gateway (M12 production transport)

```bash
cargo run -p tbc-gateway --release
```

Listens on **quic://127.0.0.1:4433** (override with `TBC_QUIC_PORT`). Ops HTTP on **http://127.0.0.1:9443** (`/health`, `/ready`, `/metrics`).

Production TLS — set PEM paths (omit for dev self-signed cert):

```bash
export TBC_TLS_CERT=/path/to/cert.pem
export TBC_TLS_KEY=/path/to/key.pem
# Optional mTLS:
# export TBC_TLS_CLIENT_CA=/path/to/client-ca.pem
./deploy/generate-dev-tls.sh deploy/tls
```

Shares `TBC_ARCHIVE_PATH` with the HTTP server. Supports `TBC_SHARD_ID` for shard-only gateway nodes.

Smoke-test client:

```bash
cargo run -p tbc-gateway --release --example quic_client
```

### Ops endpoints (M12)

| Endpoint | Server | Gateway ops |
| --- | --- | --- |
| `GET /health` | `:6014` | `:9443` |
| `GET /ready` | `:6014` | `:9443` |
| `GET /metrics` | `:6014` | `:9443` |

See **ops/RUNBOOK.md** for probes, Docker Compose, and incident playbooks.

### Docker Compose

```bash
cd deploy
cp env.example .env
./generate-dev-tls.sh tls
docker compose up --build
```

### Run tests

```bash
cargo test -p tbc-engine
```

35+ tests cover core simulation, M8–M16.

```bash
cargo test -p tbc-engine scale_guardrails
cargo test -p tbc-engine persist_hydrate
cargo test -p tbc-engine shard_multinode
cargo test -p tbc-engine ops_health
cargo test -p tbc-engine gameplay_ruleset
cargo test -p tbc-engine npmr_dream
cargo test -p tbc-engine consent_wire
cargo test -p tbc-engine psi_social
cargo test -p tbc-engine assist
# With NATS running:
cargo test -p tbc-engine nats_publish_roundtrip -- --ignored
```

## Architecture

```
AUM_Core
├── SoulArchive (SQLite)   durable IUOC + experience packets (M9)
├── IUOCRegistry         hydrated from archive on boot
├── EntropyLedger        private quality scalar (S)
├── RwwBus               memory cache + NATS JetStream replication (M10)
├── Frame PMR shard-00   Δt=50ms, x ∈ [-500, 50]  (or standalone node TBC_SHARD_ID=0)
├── Frame PMR shard-01   Δt=50ms, x ∈ [-50, 500]  (or standalone node TBC_SHARD_ID=1)
│   ├── IslandManager    clustering, observe-collapse, profiler
│   ├── GuardrailState   rate limits + budget enforcement
│   ├── NetcodeState     snapshot ring, rewind-replay
│   └── HierGrid         render-on-observation
├── Frame NPMR-Academy   Δt=200ms, blink, CRDT props
├── Frame NPMR-Dream     Δt=200ms, looser ruleset, dream echoes
└── Reincarnation planner  K=5 ranked packet templates

Transport
├── tbc-server           HTTP/WebSocket (debug UI) + /health /ready /metrics
└── tbc-gateway          QUIC TLS/mTLS + ops HTTP :9443
```

## Milestones

| Milestone | Status |
| --- | --- |
| M1 Tick + ledger | Done |
| M2 Soul bind | Done |
| M3 Observe (grid, sleep/wake) | Done |
| M4 Netcode (rewind, speed hack) | Done |
| M5 NPMR + RWW + psi | Done |
| M6 Island beam profiler | Done |
| M7 Seamless multi-shard PMR + reincarnation | Done |
| M8 Guardrails + scale test | Done |
| M9 Persistent IUOC + experience archive | Done |
| M10 Real RWW (NATS JetStream) | Done |
| M11 Multi-node PMR sharding | Done |
| M12 Production transport + ops | Done |
| M13 Gameplay depth (ruleset-driven) | Done |
| M14 NPMR Dream + CRDT props + handoff policy | Done |
| M15 Psi scopes + Speak + Consent | Done |
| M16 Consent-gated assist + CI | Done |
| M17 Typed client SDK (`tbc-sdk`) | Done |
| M18 Consent-stamped wire intents | Done |
| M19 Release + crates.io publish | Done |

## Publishing (M19)

Published crates: **`tbc-engine`** (simulation core) and **`tbc-sdk`** (HTTP client). Application binaries `tbc-server` and `tbc-gateway` are workspace-only (`publish = false`).

```bash
# Verify packaging locally (no token required)
cargo publish -p tbc-engine --dry-run
# After tbc-engine is on crates.io:
cargo publish -p tbc-sdk --dry-run
```

To publish from CI, add a `CARGO_REGISTRY_TOKEN` secret and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The **Release** workflow runs tests, `cargo publish` for both crates, and creates a GitHub release with `CHANGELOG.md`. See [CHANGELOG.md](CHANGELOG.md) for version history.

## Client SDK (M17–M18)

Rust crate `tbc-sdk` wraps the debug HTTP API and re-exports QUIC wire types:

```bash
cargo run -p tbc-sdk --example http_demo
# TBC_URL=http://127.0.0.1:6014 cargo run -p tbc-sdk --example http_demo
```

Browser: `web/sdk/tbc-client.js` — `TbcClient` class used by the debug UI.

Wire message builders (`assist`, `speak`, `psi`, `handoff`, …) live in `tbc_engine::transport` for QUIC gateway clients. M18 adds optional `consent` stamps on `assist` / `speak` payloads and `wire_to_intent()` for `Verb::Assist` / `Verb::Speak`.

## Rulesets

- `rulesets/pmr.v1.json` — tight PMR-Prime (20 Hz, gravity, conserved items)
- `rulesets/npmr.academy.v1.json` — loose NPMR (blink, CRDT props)
- `rulesets/npmr.dream.v1.json` — deepest NPMR (blink, dream props, handoff hub)

## Lore & design

Based on Thomas Campbell's *My Big TOE*. Consciousness is fundamental; PMR is computed virtual reality served to FWAU observers. Entropy measures quality of consciousness — not XP. There is no public virtue leaderboard.

## License

Apache-2.0
