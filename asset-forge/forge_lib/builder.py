from __future__ import annotations

from typing import Any

from forge_lib.generate import merge_sprite_controls
from forge_lib.sprite_gen import SpriteSpec, stable_seed
from forge_lib.templates import apply_archetype


def draft_defaults() -> dict[str, Any]:
    entity = apply_archetype("mentor", "npc.new_entity", "New Entity")
    entity["transform_default"]["position"] = {"x": 0.0, "y": 0.0, "z": 0.0}
    return entity


def build_entity_from_controls(
    *,
    archetype: str,
    entity_id: str,
    display_name: str,
    lore: str,
    tags_csv: str,
    kind: str,
    interactable: bool,
    hp: float,
    stamina: float,
    brain_kind: str,
    brain_seed: int,
    pos_x: float,
    pos_y: float,
    pos_z: float,
    rotation: float,
    shape: str,
    pattern: str,
    glyph: str,
    primary: str,
    accent: str,
    glow: float,
    sprite_size: int,
    outline: bool,
    art_seed: int,
) -> dict[str, Any]:
    base = apply_archetype(
        archetype,
        entity_id,
        display_name,
        position={"x": pos_x, "y": pos_y, "z": pos_z},
        rotation=rotation,
        sprite_seed=art_seed,
    )
    base["kind"] = kind
    base["lore"] = lore
    base["tags"] = [t.strip() for t in tags_csv.split(",") if t.strip()]
    base["interactable"] = interactable
    if kind != "player_spawn":
        base["avatar"] = {"hp": hp, "stamina": stamina}
        base["brain"] = {"kind": brain_kind, "seed": brain_seed}
    else:
        base.pop("avatar", None)
        base.pop("brain", None)

    slug = entity_id.split(".")[-1]
    base["sprite"]["path"] = f"sprites/{slug}.png"
    return merge_sprite_controls(
        base,
        shape=shape,
        pattern=pattern,
        glyph=glyph,
        primary=primary,
        accent=accent,
        glow=glow,
        size=sprite_size,
        outline=outline,
        seed=art_seed,
    )


def preview_spec_from_entity(entity: dict[str, Any]) -> SpriteSpec:
    from forge_lib.sprite_gen import spec_from_entity

    spec = spec_from_entity(entity)
    if spec:
        return spec
    return SpriteSpec(seed=stable_seed(entity.get("id", "x")))
