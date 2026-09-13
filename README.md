# The Big Computer (TBC) Engine

Rust library: AUM core, PMR/NPMR frames, entropy ledger, netcode rewind, sharding, ruleset verbs, wire helpers.

## Layout

| Path | Purpose |
| --- | --- |
| `tbc-engine/` | Crate source, tests, `tick_step` bench |
| `rulesets/*.json` | PMR / NPMR Academy / NPMR Dream rules |

## Play the game (`tbc-play`)

Native client that lists **all playable levels**: built-in ruleset sandboxes plus every content pack under `assets/packs/`.

**Windows:** double-click [`run-play.bat`](run-play.bat), or:

```bash
cargo run -p tbc-game --release
```

Controls: **↑↓** level select · **Enter** play · **WASD** move · **E** interact · **Space** attack · **Esc** back to menu.

Set `HADES_REPO_ROOT` if you launch the binary from outside the repo (must contain `assets/packs` and `rulesets`).

## Mechanics Forge

Design **game mechanics**: browse what `tbc-engine` implements (verbs, motion, psi, death, handoffs), edit **ruleset JSON** visually, and save **custom extensions** (patch bundles + design notes) under `mechanics/extensions/`.

**Windows:** double-click [`run-mechanics-forge.bat`](run-mechanics-forge.bat) (port **8503**). Workbench UI: section forms with **Apply**, undo/redo, baseline diff, ruleset compare, JSON source mode, and patch-table extensions.

## Asset Forge

Standalone **Asset Studio** to author full game assets in-app (procedural sprites, stats, placement, lore) and export content packs under `assets/`, plus ruleset fork/validate.

**Windows:** double-click [`run-asset-forge.bat`](run-asset-forge.bat) (port **8502**), or:

```bash
cd asset-forge && python cli.py init-starter && python cli.py validate
```

## Local dev dashboard

Track and run the same checks as CI from your machine (tests, release build, fmt, clippy, packaging dry-run). History is stored under `dashboard/.data/`.

**Windows:** double-click [`run-dashboard.bat`](run-dashboard.bat) in the repo root, or run `.\run-dashboard.ps1`. Opens http://localhost:8501 (needs Python 3.10+ and `cargo` on PATH).

## Commands

```bash
cargo test -p tbc-engine
cargo build -p tbc-engine --release
cargo bench -p tbc-engine --bench tick_step
cargo publish -p tbc-engine --dry-run
```

Optional **NATS JetStream** for distributed RWW (otherwise in-memory):

```bash
cargo build -p tbc-engine --features nats
# TBC_NATS_URL=nats://127.0.0.1:4222 cargo test -p tbc-engine --features nats nats_publish_roundtrip -- --ignored
```

Release profile: fat LTO, single codegen unit, strip.

## API entry

- `tbc_engine::aum::AumCore` — boot cluster/shard, bind souls, run ticks, handoffs, reincarnation
- `tbc_engine::frame::Frame` — single simulation frame
- `tbc_engine::transport` — wire messages + `wire_to_intent`
- `RulesetRegistry::load_dir("rulesets")` — JSON rules

Apache-2.0 — see [LICENSE](LICENSE), [CHANGELOG](CHANGELOG.md).

**GitHub:** metadata points at [alexdepasquale2-jpg/hades-engine](https://github.com/alexdepasquale2-jpg/hades-engine). First-time publish steps: [docs/GITHUB_SETUP.md](docs/GITHUB_SETUP.md).
