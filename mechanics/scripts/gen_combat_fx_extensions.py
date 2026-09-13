"""Generate mechanics/extensions/custom.combat_fx_<profile>.json from catalog."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
FORGE = REPO / "mechanics-forge"
sys.path.insert(0, str(FORGE))

from mech_lib.extensions import validate_extension

CATALOG = REPO / "mechanics/catalog/combat_fx_presets.json"
EXT_DIR = REPO / "mechanics/extensions"

HINTS: dict[str, dict] = {
    "crisp_hitstop": {"verbs.attack.harm_entropy": 0.55, "tightness": 0.9},
    "heavy_impact": {"verbs.attack.damage": 44, "verbs.attack.harm_entropy": 0.62, "tightness": 0.94},
    "slash_ribbon": {"verbs.attack.stamina_cost": 10, "motion.max_speed": 7.5},
    "psi_afterimage": {"psi.enabled": True, "verbs.attack.harm_entropy": 0.48, "tightness": 0.75},
    "elemental_burst_fire": {"verbs.attack.damage": 38, "verbs.attack.range_m": 7},
    "elemental_burst_frost": {"verbs.attack.damage": 32, "verbs.attack.harm_entropy": 0.42},
    "parry_spark": {"verbs.attack.stamina_cost": 14, "tightness": 0.96},
    "execute_finisher": {"verbs.attack.damage": 50, "verbs.attack.harm_entropy": 0.7, "death.rewind_s": 2},
    "ranged_tracer": {"verbs.attack.range_m": 14, "verbs.attack.damage": 28},
    "aoe_shockwave": {"verbs.attack.range_m": 10, "verbs.attack.damage": 30},
    "lifesteal_glow": {"verbs.assist.enabled": True, "verbs.assist.heal_amount": 12},
    "combo_roulette": {"verbs.attack.stamina_cost": 9, "motion.max_speed": 7.2},
    "boss_phase_break": {"verbs.attack.damage": 46, "tightness": 0.97},
    "screen_kick_punch": {"verbs.attack.harm_entropy": 0.58, "motion.max_speed": 6.8},
    "minimal_tactical": {"verbs.attack.harm_entropy": 0.35, "tightness": 0.99, "dt_ms": 50},
    "entropy_glyph_storm": {"psi.enabled": True, "verbs.attack.harm_entropy": 0.52},
    "assist_synergy_burst": {"verbs.assist.enabled": True, "verbs.assist.range_m": 8},
    "death_rewind_glitch": {"death.rewind_s": 3, "verbs.attack.harm_entropy": 0.65},
    "stamina_rhythm": {"verbs.attack.stamina_cost": 11},
    "multi_hit_flurry": {"verbs.attack.damage": 22, "verbs.attack.stamina_cost": 8},
    "block_absorb": {"verbs.attack.stamina_cost": 15, "tightness": 0.93},
    "ambient_dust_impact": {"motion.gravity": 9.8, "verbs.attack.harm_entropy": 0.45},
    "neon_arcade": {"tightness": 0.7, "verbs.attack.damage": 36},
    "silent_killer": {"verbs.attack.damage": 40, "verbs.attack.harm_entropy": 0.4},
}


def main() -> None:
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    for preset in catalog["presets"]:
        pid = preset["profile_id"]
        eid = f"custom.combat_fx_{pid}"
        patches = dict(HINTS.get(pid, {}))
        patches["presentation.combat"] = preset
        ext = {
            "id": eid,
            "version": 1,
            "title": f"Combat FX — {preset['title']}",
            "description": f'Apply graphical combat profile "{pid}" plus tuned attack feel.',
            "category": "combat",
            "engine_status": "ruleset_patch",
            "patches": patches,
            "combat_presentation": preset,
            "design_notes": [
                f"Profile: {pid}. Tags: {', '.join(preset.get('tags', []))}.",
                "Merge into draft in Mechanics Forge → Extensions → Apply patches to ruleset draft.",
            ],
            "compatible_rulesets": ["pmr.v1"],
        }
        path = EXT_DIR / f"{eid.replace('.', '_')}.json"
        path.write_text(json.dumps(ext, indent=2) + "\n", encoding="utf-8")
        errs = validate_extension(ext)
        if errs:
            raise SystemExit(f"{path.name}: {errs}")
        print(path.name)


if __name__ == "__main__":
    main()
