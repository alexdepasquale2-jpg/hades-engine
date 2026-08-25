# The Big Computer (TBC) Engine

An MBT-native MMORPG engine implementing the architecture from **The Big Computer Engine Specification** — consciousness-first simulation where AUM_Core is authority, TBC renders PMR one Δt at a time, and the EntropyLedger is the private character sheet.

## What this repo implements

**M1–M7 vertical slice** of the spec:

| Module | Spec section | Status |
| --- | --- | --- |
| `DeltaTClock` | §5 Δt | Fixed 50 ms timestep, accumulator, max 4 catch-up |
| `EntropyLedger` | §8 | Private S scalar, delayed noisy consequences |
| `IUOC` / `FWAU` | §3 | Soul bind, quality snapshot, experience packets |
| `HierGrid` | §4 | 32/128/512 m spatial hash, interest queries |
| `ProbabilitySurface` | §7 | Beam B=16, D=8, A=6 intent-biased prune |
| `IslandManager` | §7 M6 | ~40 islands, observe-collapse, step profiler |
| `NetcodeState` | §7 | Snapshot ring N=8, rewind-replay, corrections |
| `ShardBounds` | §10 M7 | PMR shard-00 / shard-01, overlap strip handoff |
| `Reincarnation planner` | §11 M7 | K=5 ranked offers, accept + rebirth |
| `RwwBus` | §9 | In-memory RWW fabric (NATS-ready subjects) |
| `transport` | §10 | Framed JSON reliable + 36-byte move datagrams |
| `Frame` PMR + NPMR | §4–6 | Dual PMR shards + NPMR Academy |
| Debug server | §10 | HTTP + WebSocket on port **6014** |
| QUIC gateway | §10 | `tbc-gateway` on **4433** (quinn) |

## Quick start

```bash
cargo run -p tbc-server --release
```

Open **http://127.0.0.1:6014**

1. **Partition FWAU** — bind an IUOC avatar on PMR shard-00 (west)
2. **WASD** — move; walk **east (→)** past the yellow seam at x=0 to trigger seamless shard handoff to shard-01
3. **Island profiler** — sidebar shows island count and beam step budget (M6)
4. **Unbind / death** — between-lives flow; pick one of five reincarnation offers (M7)
5. **Enter NPMR-Academy** — frame handoff via RWW (ruleset change, not spatial shard)
6. **B** — blink in NPMR (loose ruleset)
7. **FutureSelf / PastOwn** — psi queries against beam and packet archive

### QUIC gateway (transport layer)

```bash
cargo run -p tbc-gateway --release
```

Listens on **quic://127.0.0.1:4433** with a self-signed cert.

- **Reliable bi-stream**: length-prefixed JSON (`login`, `move`, `snapshot`)
- **Unreliable datagrams**: 36-byte move packets (`encode_move_datagram`)

Smoke-test client:

```bash
cargo run -p tbc-gateway --release --example quic_client
```

### Run tests

```bash
cargo test -p tbc-engine
```

17 tests cover Δt, ledger, grid, beam budgets, islands profiler, shard overlap, netcode rewind, transport wire format, planner offers, and RWW publish.

## Architecture

```
AUM_Core
├── IUOCRegistry      durable souls + experience packets
├── EntropyLedger     private quality scalar (S)
├── RwwBus            in-memory RWW (rww.bound.*, rww.handoff)
├── Frame PMR shard-00   Δt=50ms, x ∈ [-500, 50]
├── Frame PMR shard-01   Δt=50ms, x ∈ [-50, 500]
│   ├── IslandManager   clustering, observe-collapse, profiler
│   ├── NetcodeState    snapshot ring, rewind-replay
│   └── HierGrid        render-on-observation
├── Frame NPMR-Academy   Δt=200ms, blink, loose ruleset
└── Reincarnation planner  K=5 ranked packet templates

Transport
├── tbc-server          HTTP/WebSocket (debug UI)
└── tbc-gateway         QUIC (quinn) reliable + datagram moves
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
| M8 Guardrails + scale test | Planned |

## Rulesets

- `rulesets/pmr.v1.json` — tight PMR-Prime (20 Hz, gravity, conserved items)
- `rulesets/npmr.academy.v1.json` — loose NPMR (blink, CRDT props)

## Lore & design

Based on Thomas Campbell's *My Big TOE*. Consciousness is fundamental; PMR is computed virtual reality served to FWAU observers. Entropy measures quality of consciousness — not XP. There is no public virtue leaderboard.

## License

Apache-2.0
