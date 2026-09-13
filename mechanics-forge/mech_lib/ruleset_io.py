from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path
from typing import Any

from mech_lib.paths import RULESETS_DIR

REQUIRED_KEYS = (
    "id",
    "title",
    "tightness",
    "dt_ms",
    "clock",
    "motion",
    "conservation",
    "death",
    "psi",
    "sleep",
    "handoff",
    "verbs",
)
VERB_KEYS = ("attack", "interact", "speak", "assist")

PSI_SCOPES = [
    "PastOwn",
    "PastShared",
    "FutureSelf",
    "FutureIsland",
    "RwwQuery",
]


def list_ruleset_files() -> list[Path]:
    if not RULESETS_DIR.is_dir():
        return []
    return sorted(RULESETS_DIR.glob("*.json"))


def list_ruleset_ids() -> list[str]:
    return [p.stem for p in list_ruleset_files()]


def load_ruleset_by_id(ruleset_id: str) -> dict[str, Any]:
    path = RULESETS_DIR / f"{ruleset_id}.json"
    return json.loads(path.read_text(encoding="utf-8"))


def load_ruleset_path(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def save_ruleset(data: dict[str, Any]) -> Path:
    rid = data.get("id", "unknown")
    path = RULESETS_DIR / f"{rid}.json"
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return path


def validate_ruleset(data: dict[str, Any], source: str = "draft") -> list[str]:
    errors: list[str] = []
    for key in REQUIRED_KEYS:
        if key not in data:
            errors.append(f"{source}: missing `{key}`")
    verbs = data.get("verbs")
    if isinstance(verbs, dict):
        for vk in VERB_KEYS:
            if vk not in verbs:
                errors.append(f"{source}: verbs missing `{vk}`")
    if not data.get("id"):
        errors.append(f"{source}: empty id")
    return errors


def ensure_verb_defaults(verbs: dict[str, Any]) -> dict[str, Any]:
    verbs = deepcopy(verbs)
    verbs.setdefault(
        "attack",
        {
            "enabled": True,
            "damage": 34,
            "range_m": 8,
            "stamina_cost": 12,
            "harm_entropy": 0.5,
        },
    )
    verbs.setdefault(
        "interact",
        {"enabled": True, "range_m": 4, "item_prefix": "echo"},
    )
    verbs.setdefault("speak", {"enabled": True, "range_m": 24})
    verbs.setdefault(
        "assist",
        {
            "enabled": False,
            "range_m": 6,
            "stamina_cost": 8,
            "heal_amount": 25,
            "aid_entropy": 0.35,
            "ai_practice": False,
        },
    )
    return verbs


def blank_from_template(base_id: str, new_id: str, title: str) -> dict[str, Any]:
    data = load_ruleset_by_id(base_id)
    data = deepcopy(data)
    data["id"] = new_id
    data["title"] = title
    return data


def set_path(data: dict[str, Any], dotted: str, value: Any) -> None:
    parts = dotted.split(".")
    cur: Any = data
    for part in parts[:-1]:
        if part not in cur or not isinstance(cur[part], dict):
            cur[part] = {}
        cur = cur[part]
    cur[parts[-1]] = value


def apply_patches(data: dict[str, Any], patches: dict[str, Any]) -> dict[str, Any]:
    out = deepcopy(data)
    for path, value in patches.items():
        set_path(out, path, value)
    return out
