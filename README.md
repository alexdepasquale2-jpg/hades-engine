# The Big Computer (TBC) Engine

An MBT-native MMORPG engine implementing the architecture from **The Big Computer Engine Specification** — consciousness-first simulation where AUM_Core is authority, TBC renders PMR one Δt at a time, and the EntropyLedger is the private character sheet.

## What this repo implements

This is the **M1–M3 vertical slice** of the spec:

| Module | Spec section | Status |
| --- | --- | --- |
| `DeltaTClock` | §5 Δt | Fixed 50 ms timestep, accumulator, max 4 catch-up |
| `EntropyLedger` | §8 | Private S scalar, delayed noisy consequences |
| `IUOC` / `FWAU` | §3 | Soul bind, quality snapshot, experience packets |
| `HierGrid` | §4 | 32/128/512 m spatial hash, interest queries |
| `ProbabilitySurface` | §7 | Beam B=16, D=8, A=6 intent-biased prune |
| `Frame` | §4–5 | 20 Hz sim loop, sleep/wake, AI Guys, replication |
| `Ruleset` | §6 | PMR-Prime + NPMR-Academy data shapes |
| Debug server | §10 gateway | HTTP + WebSocket on port 6014 |

## Quick start

### Requirements

- Rust 1.75+ (`rustup` recommended)

### Run the debug server

```bash
cargo run -p tbc-server --release
```

Open **http://127.0.0.1:6014**

1. Click **Partition FWAU** to create an IUOC and bind an avatar.
2. Use **WASD** or arrow keys to move (intent verbs on the unreliable path).
3. Click **Query probable futures** for a FutureSelf psi read from the live beam.

### Run tests

```bash
cargo test -p tbc-engine
```

Tests verify Δt accumulator behavior, entropy scoring direction, O(1) grid moves, and beam step budgets from the spec.

## Architecture

```
AUM_Core
├── IUOCRegistry      durable souls
├── EntropyLedger     private quality scalar (S)
└── Frame PMR-Prime   Δt=50ms, ruleset=pmr.v1
    ├── ECS World     avatars + AI Guys
    ├── HierGrid      render-on-observation interest
    └── ProbabilitySurface per island
```

## Rulesets

- `rulesets/pmr.v1.json` — tight PMR-Prime (20 Hz, gravity, conserved items)
- `rulesets/npmr.academy.v1.json` — loose NPMR (blink, CRDT props)

## Roadmap (from spec §14)

- **M4** — QUIC gateway (quinn), rewind-replay netcode
- **M5** — NATS RWW bus, NPMR Academy frame
- **M6** — Full island beam profiler at 40 islands
- **M7** — Seamless multi-shard PMR, UE5 reference client

## Lore & design

Based on Thomas Campbell's *My Big TOE*. Consciousness is fundamental; PMR is computed virtual reality served to FWAU observers. Entropy measures quality of consciousness — not XP. There is no public virtue leaderboard.

## License

Apache-2.0
