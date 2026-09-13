from __future__ import annotations

from pathlib import Path

from PIL import Image

from forge_lib.entities import load_entity
from forge_lib.paths import PACKS_DIR
from forge_lib.sprite_gen import render_sprite, spec_from_entity, stable_seed

# Legacy simple sprites (re-exported for compatibility)
from forge_lib.sprite_gen import random_sprite_spec  # noqa: F401

KIND_COLORS = {
    "npc": (72, 118, 186),
    "prop": (120, 92, 72),
    "pickup": (186, 152, 72),
    "player_spawn": (72, 186, 118),
}


def render_placeholder_sprite(
    display_name: str,
    kind: str,
    entity_id: str,
    size: int = 32,
) -> Image.Image:
    from forge_lib.sprite_gen import SpriteSpec, render_sprite as _render

    spec = random_sprite_spec(stable_seed(entity_id), kind, size=size)
    return _render(spec, display_name)


def render_entity_sprite(entity: dict) -> Image.Image:
    spec = spec_from_entity(entity)
    name = entity.get("display_name", "?")
    if spec:
        return render_sprite(spec, name)
    return render_placeholder_sprite(
        name,
        entity.get("kind", "npc"),
        entity.get("id", "id"),
        size=int((entity.get("sprite") or {}).get("size", 32)),
    )


def write_entity_sprite(pack_id: str, entity: dict) -> Path:
    sprite = entity.get("sprite") or {}
    rel = sprite.get("path", f"sprites/{entity['id'].split('.')[-1]}.png")
    pack_dir = PACKS_DIR / pack_id
    out = pack_dir / rel
    out.parent.mkdir(parents=True, exist_ok=True)
    img = render_entity_sprite(entity)
    img.save(out)
    return out


def generate_sprite_for_entity(pack_id: str, entity_path: Path) -> Path:
    entity = load_entity(entity_path)
    return write_entity_sprite(pack_id, entity)


def generate_all_sprites(pack_id: str) -> list[Path]:
    from forge_lib.entities import list_entities_in_pack

    written: list[Path] = []
    for entity_path in list_entities_in_pack(pack_id):
        written.append(generate_sprite_for_entity(pack_id, entity_path))
    return written
