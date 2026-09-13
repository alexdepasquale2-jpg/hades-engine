# The Big Computer (TBC) Engine

An MBT-native MMORPG **simulation library** implementing the architecture from **The Big Computer Engine Specification** — consciousness-first simulation where AUM_Core is authority, TBC renders PMR one Δt at a time, and the EntropyLedger is the private character sheet.

This repository contains **`tbc-engine`** only: the deterministic core, rulesets, integration tests, and benchmarks. Host a game by linking this crate from your own server or client runtime.

## What the library implements

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
| `assist` | M16/M20 | Consent-gated heal + NPMR AI practice mode (intent queue) |
| `speak` | M15/M20 | Ruleset-gated social broadcast (intent queue) |
| `OpsSnapshot` | M12 | Serializable health/ready/metrics snapshot (for your host) |
| `Frame` PMR + NPMR | §4–6 | Dual PMR shards + NPMR Academy/Dream |

## Quick start (library)

Add to your `Cargo.toml`:

```toml
tbc-engine = "0.1"
```

Or develop from this repo:

```bash
cargo test -p tbc-engine
cargo build -p tbc-engine --release
```

Load rulesets from `rulesets/` (or your own paths) when constructing frames. Default archive path in tests is often a temp SQLite file; production hosts choose `SoulArchive` paths.

Optional NATS JetStream for real-world-wide RWW (integration tests use `#[ignore]` unless NATS is up):

```bash
docker run --rm -p 4222:4222 nats:2.10 -js
export TBC_NATS_URL=nats://127.0.0.1:4222
cargo test -p tbc-engine nats_publish_roundtrip -- --ignored
```

Without `TBC_NATS_URL`, `RwwBus` stays in-memory.

### Performance (release)

Release builds use **fat LTO**, **single codegen unit**, and **strip**. Hot paths reuse tick scratch buffers, O(1) FWAU→avatar lookup, beam `mem::swap`, and O(n) spatial island clustering.

```bash
cargo bench -p tbc-engine --bench tick_step
# Example: ~1300+ sim ticks/s with 200 awake AI on one PMR frame (release, hardware-dependent)
```

### Tests

35+ integration tests cover simulation, guardrails, persistence, sharding, gameplay rulesets, psi/social/consent, assist, and intent verbs:

```bash
cargo test -p tbc-engine
cargo test -p tbc-engine scale_guardrails
cargo test -p tbc-engine persist_hydrate
cargo test -p tbc-engine shard_multinode
cargo test -p tbc-engine gameplay_ruleset
cargo test -p tbc-engine npmr_dream
cargo test -p tbc-engine consent_wire
cargo test -p tbc-engine psi_social
cargo test -p tbc-engine assist
cargo test -p tbc-engine intent_verbs
```

## Architecture

```
AUM_Core (tbc-engine)
├── SoulArchive (SQLite)   durable IUOC + experience packets (M9)
├── IUOCRegistry         hydrated from archive on boot
├── EntropyLedger        private quality scalar (S)
├── RwwBus               memory cache + NATS JetStream replication (M10)
├── Frame PMR shard-00   Δt=50ms, x ∈ [-500, 50]
├── Frame PMR shard-01   Δt=50ms, x ∈ [-50, 500]
│   ├── IslandManager    clustering, observe-collapse, profiler
│   ├── GuardrailState   rate limits + budget enforcement
│   ├── NetcodeState     snapshot ring, rewind-replay
│   └── HierGrid         render-on-observation
├── Frame NPMR-Academy   Δt=200ms, blink, CRDT props
├── Frame NPMR-Dream     Δt=200ms, looser ruleset, dream echoes
└── Reincarnation planner  K=5 ranked packet templates

Wire helpers in `tbc_engine::transport` (QUIC/HTTP hosts build on these).
M18: optional `consent` stamps on assist/speak payloads and `wire_to_intent()`.
M20: Assist/Speak through netcode intent queue + rewind-replay.
```

## Milestones (engine)

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
| M12 Ops snapshot types + transport | Done |
| M13 Gameplay depth (ruleset-driven) | Done |
| M14 NPMR Dream + CRDT props + handoff policy | Done |
| M15 Psi scopes + Speak + Consent | Done |
| M16 Consent-gated assist + CI | Done |
| M18 Consent-stamped wire intents | Done |
| M19 Release + crates.io publish | Done |
| M20 Assist/Speak intent queue + rewind | Done |

## Publishing

The published crate is **`tbc-engine`**.

```bash
cargo publish -p tbc-engine --dry-run
```

To publish from CI, add a `CARGO_REGISTRY_TOKEN` secret and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The **Release** workflow runs tests, publishes `tbc-engine`, and creates a GitHub release with `CHANGELOG.md`. See [CHANGELOG.md](CHANGELOG.md).

## Rulesets

Shipped JSON (loaded by your host or tests):

- `rulesets/pmr.v1.json` — tight PMR-Prime (20 Hz, gravity, conserved items)
- `rulesets/npmr.academy.v1.json` — loose NPMR (blink, CRDT props)
- `rulesets/npmr.dream.v1.json` — deepest NPMR (blink, dream props, handoff hub)

## Lore & design

Based on Thomas Campbell's *My Big TOE*. Consciousness is fundamental; PMR is computed virtual reality served to FWAU observers. Entropy measures quality of consciousness — not XP. There is no public virtue leaderboard.

## License

Apache-2.0
