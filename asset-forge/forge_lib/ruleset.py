from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from forge_lib.paths import RULESETS_DIR

RULESET_REQUIRED_KEYS = (
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


def load_ruleset(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_ruleset_data(data: dict[str, Any], source: str) -> list[str]:
    errors: list[str] = []
    for key in RULESET_REQUIRED_KEYS:
        if key not in data:
            errors.append(f"{source}: missing required key `{key}`")
    if "verbs" in data and isinstance(data["verbs"], dict):
        for vk in VERB_KEYS:
            if vk not in data["verbs"]:
                errors.append(f"{source}: verbs missing `{vk}`")
    rid = data.get("id")
    if isinstance(rid, str) and not rid:
        errors.append(f"{source}: empty ruleset id")
    return errors


def validate_all_rulesets() -> list[str]:
    errors: list[str] = []
    if not RULESETS_DIR.is_dir():
        return [f"Rulesets directory missing: {RULESETS_DIR}"]
    for path in sorted(RULESETS_DIR.glob("*.json")):
        try:
            data = load_ruleset(path)
        except json.JSONDecodeError as exc:
            errors.append(f"{path.name}: invalid JSON ({exc})")
            continue
        errors.extend(validate_ruleset_data(data, path.name))
    return errors


def scaffold_ruleset_variant(
    base_id: str,
    new_id: str,
    title: str,
    *,
    tightness_delta: float = 0.0,
) -> Path:
    base_path = RULESETS_DIR / f"{base_id}.json"
    if not base_path.is_file():
        raise FileNotFoundError(f"Base ruleset not found: {base_path}")
    data = load_ruleset(base_path)
    data["id"] = new_id
    data["title"] = title
    if tightness_delta:
        data["tightness"] = round(float(data.get("tightness", 0.9)) + tightness_delta, 3)
    out = RULESETS_DIR / f"{new_id}.json"
    out.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return out
