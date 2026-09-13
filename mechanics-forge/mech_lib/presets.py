from __future__ import annotations

from typing import Any

PRESETS: dict[str, dict[str, Any]] = {
    "PMR — combat tuned": {
        "verbs.attack.damage": 42,
        "verbs.attack.stamina_cost": 14,
        "motion.max_speed": 6.5,
    },
    "PMR — exploration": {
        "verbs.attack.enabled": False,
        "verbs.interact.range_m": 6,
        "motion.max_speed": 8.5,
        "psi.enabled": True,
    },
    "Academy — social": {
        "verbs.speak.range_m": 64,
        "verbs.assist.enabled": True,
        "verbs.assist.ai_practice": True,
        "verbs.attack.enabled": False,
    },
    "Dream — low gravity": {
        "motion.gravity": 0.0,
        "motion.blink": True,
        "motion.max_speed": 14,
        "tightness": 0.25,
    },
    "Hardcore — lethal": {
        "verbs.attack.damage": 55,
        "verbs.attack.harm_entropy": 0.75,
        "death.unbind": True,
        "death.rewind_s": 0,
        "tightness": 0.98,
    },
    "Combat FX — crisp hit-stop": {
        "verbs.attack.harm_entropy": 0.55,
        "tightness": 0.9,
        "presentation.combat.profile_id": "crisp_hitstop",
        "presentation.combat.hit_stop_ms": 52,
        "presentation.combat.screen_shake": 0.22,
    },
    "Combat FX — heavy impact": {
        "verbs.attack.damage": 44,
        "verbs.attack.harm_entropy": 0.62,
        "presentation.combat.profile_id": "heavy_impact",
        "presentation.combat.hit_stop_ms": 78,
        "presentation.combat.screen_shake": 0.48,
    },
    "Combat FX — neon arcade": {
        "tightness": 0.7,
        "verbs.attack.damage": 36,
        "presentation.combat.profile_id": "neon_arcade",
        "presentation.combat.chromatic_aberration": 0.55,
    },
}


def preset_names() -> list[str]:
    return list(PRESETS.keys())
