# Game content assets

Data packs for clients and world authoring. The simulation engine loads **rulesets** from `rulesets/`; packs here describe spawnable entities, visuals, and bundle metadata.

| Path | Purpose |
| --- | --- |
| `schema/` | JSON Schema for manifests and entity defs |
| `packs/<id>/` | One content pack per folder (`manifest.json`, `entities/`, `sprites/`) |

Create and validate packs with **Asset Forge** (`run-asset-forge.bat`). Use the **Studio** tab to
build a full asset in-app: archetype templates, procedural sprites (shape/pattern/glyph/colors),
world placement, and one-click export (JSON + PNG + manifest).
