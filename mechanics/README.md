# Game mechanics

| Path | Purpose |
| --- | --- |
| `catalog/` | Reference list of engine-implemented mechanics (documentation) |
| `extensions/` | Designer-defined **custom mechanics** (ruleset patches + design notes) |
| `schema/` | JSON Schema for extensions |

Edit rulesets in `rulesets/` with **Mechanics Forge** (`run-mechanics-forge.bat`). The engine reads ruleset JSON at runtime; custom extensions are merged when you export or save a ruleset from the forge.
