from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from forge_lib.paths import PACKS_DIR, RULESETS_DIR


def list_pack_ids() -> list[str]:
    if not PACKS_DIR.is_dir():
        return []
    return sorted(
        p.name for p in PACKS_DIR.iterdir() if p.is_dir() and (p / "manifest.json").is_file()
    )


def list_ruleset_ids() -> list[str]:
    if not RULESETS_DIR.is_dir():
        return []
    return sorted(p.stem for p in RULESETS_DIR.glob("*.json"))


def create_pack(
    pack_id: str,
    title: str,
    ruleset_id: str,
    description: str = "",
) -> Path:
    if not pack_id.startswith("pack."):
        pack_id = f"pack.{pack_id}"
    pack_dir = PACKS_DIR / pack_id
    pack_dir.mkdir(parents=True, exist_ok=True)
    (pack_dir / "entities").mkdir(exist_ok=True)
    (pack_dir / "sprites").mkdir(exist_ok=True)
    manifest = {
        "id": pack_id,
        "version": 1,
        "title": title,
        "ruleset_id": ruleset_id,
        "description": description,
        "entities": [],
    }
    path = pack_dir / "manifest.json"
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return path


def load_manifest(pack_id: str) -> dict[str, Any]:
    path = PACKS_DIR / pack_id / "manifest.json"
    return json.loads(path.read_text(encoding="utf-8"))


def save_manifest(pack_id: str, manifest: dict[str, Any]) -> None:
    path = PACKS_DIR / pack_id / "manifest.json"
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def register_entity_in_manifest(pack_id: str, entity_rel_path: str) -> None:
    manifest = load_manifest(pack_id)
    rel = entity_rel_path.replace("\\", "/")
    if rel not in manifest["entities"]:
        manifest["entities"].append(rel)
        save_manifest(pack_id, manifest)


def upsert_placement(
    pack_id: str,
    entity_rel: str,
    x: float,
    y: float,
    z: float = 0.0,
) -> None:
    manifest = load_manifest(pack_id)
    rel = entity_rel.replace("\\", "/")
    placements: list[dict] = manifest.setdefault("placements", [])
    for entry in placements:
        if entry.get("entity") == rel:
            entry.update({"x": x, "y": y, "z": z})
            save_manifest(pack_id, manifest)
            return
    placements.append({"entity": rel, "x": x, "y": y, "z": z})
    save_manifest(pack_id, manifest)
