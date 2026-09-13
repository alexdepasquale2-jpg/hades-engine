# Game mechanics

| Path | Purpose |
| --- | --- |
| `catalog/` | Reference data (e.g. `combat_fx_presets.json` — 24 combat juice profiles) |
| `extensions/` | Designer-defined **custom mechanics** (ruleset patches + design notes) |
| `schema/` | JSON Schema for extensions |

Edit rulesets in `rulesets/` with **Mechanics Forge** (`run-mechanics-forge.bat`). The engine reads ruleset JSON at runtime; custom extensions are merged when you export or save a ruleset from the forge.

**Combat juice:** open `catalog/combat_fx_presets.json` for the full preset list, or load any `extensions/custom.combat_fx_<profile>.json` in the forge **Extensions** page and **Apply patches to ruleset draft**. Regenerate apply packs after editing the catalog with `python mechanics/scripts/gen_combat_fx_extensions.py`.
