from __future__ import annotations

import json
from pathlib import Path

import jsonschema

from forge_lib.entities import load_entity
from forge_lib.packs import list_pack_ids, load_manifest
from forge_lib.paths import PACKS_DIR, SCHEMA_DIR
from forge_lib.ruleset import validate_all_rulesets


def _load_schema(name: str) -> dict:
    return json.loads((SCHEMA_DIR / name).read_text(encoding="utf-8"))


def validate_entity_file(path: Path) -> list[str]:
    schema = _load_schema("entity.def.schema.json")
    errors: list[str] = []
    try:
        data = load_entity(path)
    except json.JSONDecodeError as exc:
        return [f"{path}: invalid JSON ({exc})"]
    validator = jsonschema.Draft202012Validator(schema)
    for err in sorted(validator.iter_errors(data), key=lambda e: list(e.path)):
        loc = ".".join(str(p) for p in err.path) or "(root)"
        errors.append(f"{path.name}: {loc}: {err.message}")
    return errors


def validate_pack(pack_id: str) -> list[str]:
    errors: list[str] = []
    pack_dir = PACKS_DIR / pack_id
    manifest_path = pack_dir / "manifest.json"
    if not manifest_path.is_file():
        return [f"Pack `{pack_id}` has no manifest.json"]
    schema = _load_schema("pack.manifest.schema.json")
    try:
        manifest = load_manifest(pack_id)
    except json.JSONDecodeError as exc:
        return [f"{pack_id}/manifest.json: invalid JSON ({exc})"]
    validator = jsonschema.Draft202012Validator(schema)
    for err in sorted(validator.iter_errors(manifest), key=lambda e: list(e.path)):
        loc = ".".join(str(p) for p in err.path) or "(root)"
        errors.append(f"manifest: {loc}: {err.message}")
    for rel in manifest.get("entities", []):
        ent_path = pack_dir / rel
        if not ent_path.is_file():
            errors.append(f"manifest lists missing entity file: {rel}")
            continue
        errors.extend(validate_entity_file(ent_path))
        ent = load_entity(ent_path)
        sprite_rel = (ent.get("sprite") or {}).get("path")
        if sprite_rel and not (pack_dir / sprite_rel).is_file():
            errors.append(f"{rel}: sprite missing at {sprite_rel} (run Generate sprites)")
    return errors


def validate_all() -> list[str]:
    errors = validate_all_rulesets()
    for pack_id in list_pack_ids():
        errors.extend(validate_pack(pack_id))
    return errors
