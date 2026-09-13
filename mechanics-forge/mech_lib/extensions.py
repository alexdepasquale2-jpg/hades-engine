from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import jsonschema

from mech_lib.paths import EXTENSIONS_DIR, SCHEMA_DIR


def _ensure_dir() -> None:
    EXTENSIONS_DIR.mkdir(parents=True, exist_ok=True)


def list_extensions() -> list[Path]:
    _ensure_dir()
    return sorted(EXTENSIONS_DIR.glob("*.json"))


def load_extension(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def save_extension(data: dict[str, Any]) -> Path:
    _ensure_dir()
    eid = data["id"].replace(".", "_")
    path = EXTENSIONS_DIR / f"{eid}.json"
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return path


def validate_extension(data: dict[str, Any]) -> list[str]:
    schema = json.loads(
        (SCHEMA_DIR / "custom_mechanic.schema.json").read_text(encoding="utf-8")
    )
    errors: list[str] = []
    validator = jsonschema.Draft202012Validator(schema)
    for err in sorted(validator.iter_errors(data), key=lambda e: list(e.path)):
        loc = ".".join(str(p) for p in err.path) or "(root)"
        errors.append(f"{loc}: {err.message}")
    return errors


def new_extension(
    ext_id: str,
    title: str,
    description: str = "",
    category: str = "custom",
) -> dict[str, Any]:
    if not ext_id.startswith("custom."):
        ext_id = f"custom.{ext_id}"
    return {
        "id": ext_id,
        "version": 1,
        "title": title,
        "description": description,
        "category": category,
        "engine_status": "ruleset_patch",
        "patches": {},
        "design_notes": [],
        "compatible_rulesets": [],
    }
