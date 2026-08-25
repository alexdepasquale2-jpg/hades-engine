# The Big Computer (TBC) Engine

An MBT-native MMORPG engine implementing the architecture from **The Big Computer Engine Specification** — consciousness-first simulation where AUM_Core is authority, TBC renders PMR one Δt at a time, and the EntropyLedger is the private character sheet.

## What this repo implements

**M1–M5 vertical slice** of the spec:

| Module | Spec section | Status |
| --- | --- | --- |
| `DeltaTClock` | §5 Δt | Fixed 50 ms timestep, accumulator, max 4 catch-up |
| `EntropyLedger` | §8 | Private S scalar, delayed noisy consequences |
| `IUOC` / `FWAU` | §3 | Soul bind, quality snapshot, experience packets |
| `HierGrid` | §4 | 32/128/512 m spatial hash, interest queries |
| `ProbabilitySurface` | §7 | Beam B=16, D=8, A=6 intent-biased prune |
| `NetcodeState` | §7 | Snapshot ring N=8, rewind-replay, corrections |
| `RwwBus` | §9 | In-memory RWW fabric (NATS-ready subjects) |
| `Frame` PMR + NPMR | §4–6 | Dual frames, handoff, blink, speed validation |
| Debug server | §10 | HTTP + WebSocket gateway on port 6014 |

## Quick start

```bash
cargo run -p tbc-server --release
```

Open **http://127.0.0.1:6014**

1. **Partition FWAU** — bind an IUOC avatar in PMR-Prime
2. **WASD** — move (intents with client tick for rewind testing)
3. **Enter NPMR-Academy** — frame handoff via RWW
4. **B** — blink in NPMR (loose ruleset)
5. **FutureSelf / PastOwn** — psi queries against beam and packet archive

### Run tests

```bash
cargo test -p tbc-engine
```

11 tests cover Δt, ledger, grid, beam budgets, netcode rewind, and RWW publish.

## Architecture

```
AUM_Core
├── IUOCRegistry      durable souls
├── EntropyLedger     private quality scalar (S)
├── RwwBus            in-memory RWW (rww.bound.*, rww.handoff)
├── Frame PMR-Prime   Δt=50ms, ruleset=pmr.v1
│   ├── NetcodeState  snapshot ring, rewind-replay
│   ├── HierGrid      render-on-observation
│   └── ProbabilitySurface per island
└── Frame NPMR-Academy  Δt=200ms, blink, loose ruleset
```

## Milestones

| Milestone | Status |
| --- | --- |
| M1 Tick + ledger | Done |
| M2 Soul bind | Done |
| M3 Observe (grid, sleep/wake) | Done |
| M4 Netcode (rewind, speed hack) | Done |
| M5 NPMR + RWW + psi | Done |
| M6 Island beam profiler | Next |
| M7 Seamless multi-shard PMR | Planned |
| M8 Guardrails + scale test | Planned |

## Rulesets

- `rulesets/pmr.v1.json` — tight PMR-Prime (20 Hz, gravity, conserved items)
- `rulesets/npmr.academy.v1.json` — loose NPMR (blink, CRDT props)

## Lore & design

Based on Thomas Campbell's *My Big TOE*. Consciousness is fundamental; PMR is computed virtual reality served to FWAU observers. Entropy measures quality of consciousness — not XP. There is no public virtue leaderboard.

## License

Apache-2.0
