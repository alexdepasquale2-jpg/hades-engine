from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from forge_lib.paths import PACKS_DIR


def default_entity(
    entity_id: str,
    display_name: str,
    kind: str = "npc",
    *,
    interactable: bool = True,
    hp: float = 100.0,
    stamina: float = 100.0,
    brain_kind: str = "Utility",
    brain_seed: int = 1,
) -> dict[str, Any]:
    slug = entity_id.split(".")[-1]
    return {
        "id": entity_id,
        "version": 1,
        "display_name": display_name,
        "kind": kind,
        "tags": [],
        "interactable": interactable,
        "transform_default": {
            "position": {"x": 0.0, "y": 0.0, "z": 0.0},
            "rotation": 0.0,
        },
        "avatar": {"hp": hp, "stamina": stamina},
        "brain": {"kind": brain_kind, "seed": brain_seed},
        "sprite": {"path": f"sprites/{slug}.png", "size": 32},
    }


def write_entity(pack_id: str, entity: dict[str, Any]) -> Path:
    pack_dir = PACKS_DIR / pack_id
    entities_dir = pack_dir / "entities"
    entities_dir.mkdir(parents=True, exist_ok=True)
    slug = entity["id"].split(".")[-1]
    path = entities_dir / f"{slug}.json"
    path.write_text(json.dumps(entity, indent=2) + "\n", encoding="utf-8")
    return path


def load_entity(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def list_entities_in_pack(pack_id: str) -> list[Path]:
    root = PACKS_DIR / pack_id / "entities"
    if not root.is_dir():
        return []
    return sorted(root.glob("*.json"))
