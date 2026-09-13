from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from forge_lib.sprite_gen import SpriteSpec, random_sprite_spec, stable_seed


@dataclass(frozen=True)
class Archetype:
    id: str
    label: str
    kind: str
    description: str
    tags: tuple[str, ...]
    hp: float
    stamina: float
    brain_kind: str
    interactable: bool
    glyph: str
    shape: str
    pattern: str


ARCHETYPES: dict[str, Archetype] = {
    "guard": Archetype(
        id="guard",
        label="Guard",
        kind="npc",
        description="Front-line defender with high stamina.",
        tags=("combat", "faction"),
        hp=140,
        stamina=120,
        brain_kind="Utility",
        interactable=True,
        glyph="bolt",
        shape="shield",
        pattern="rim",
    ),
    "mentor": Archetype(
        id="mentor",
        label="Mentor",
        kind="npc",
        description="Guide NPC for tutorials and quests.",
        tags=("quest", "friendly"),
        hp=100,
        stamina=80,
        brain_kind="Utility",
        interactable=True,
        glyph="star",
        shape="rounded_rect",
        pattern="pulse",
    ),
    "merchant": Archetype(
        id="merchant",
        label="Merchant",
        kind="npc",
        description="Trade and interact-focused NPC.",
        tags=("trade", "friendly"),
        hp=80,
        stamina=60,
        brain_kind="BehaviorTree",
        interactable=True,
        glyph="heart",
        shape="circle",
        pattern="stripes",
    ),
    "echo_gate": Archetype(
        id="echo_gate",
        label="Echo Gate",
        kind="prop",
        description="Resonant structure tied to echo items.",
        tags=("echo", "structure"),
        hp=1,
        stamina=0,
        brain_kind="Utility",
        interactable=True,
        glyph="none",
        shape="hex",
        pattern="rim",
    ),
    "loot_shard": Archetype(
        id="loot_shard",
        label="Loot shard",
        kind="pickup",
        description="Collectible pickup for inventories.",
        tags=("loot", "pickup"),
        hp=1,
        stamina=0,
        brain_kind="Utility",
        interactable=True,
        glyph="star",
        shape="diamond",
        pattern="noise",
    ),
    "spawn_point": Archetype(
        id="spawn_point",
        label="Player spawn",
        kind="player_spawn",
        description="Default entry point for avatars.",
        tags=("spawn"),
        hp=100,
        stamina=100,
        brain_kind="Utility",
        interactable=False,
        glyph="letter",
        shape="circle",
        pattern="pulse",
    ),
}


def apply_archetype(
    archetype_key: str,
    entity_id: str,
    display_name: str,
    *,
    position: dict[str, float] | None = None,
    rotation: float = 0.0,
    sprite_seed: int | None = None,
) -> dict[str, Any]:
    arch = ARCHETYPES[archetype_key]
    slug = entity_id.split(".")[-1]
    seed = sprite_seed if sprite_seed is not None else stable_seed(entity_id)
    spec = random_sprite_spec(seed, arch.kind)
    spec.shape = arch.shape
    spec.pattern = arch.pattern
    spec.glyph = arch.glyph
    spec.seed = seed

    pos = position or {"x": 0.0, "y": 0.0, "z": 0.0}
    entity: dict[str, Any] = {
        "id": entity_id,
        "version": 1,
        "display_name": display_name,
        "kind": arch.kind,
        "tags": list(arch.tags),
        "interactable": arch.interactable,
        "transform_default": {"position": pos, "rotation": rotation},
        "avatar": {"hp": arch.hp, "stamina": arch.stamina},
        "brain": {"kind": arch.brain_kind, "seed": seed % 1_000_000},
        "sprite": {"path": f"sprites/{slug}.png", "size": spec.size},
        "sprite_gen": spec.to_dict(),
        "archetype": archetype_key,
        "lore": arch.description,
    }
    if arch.kind == "player_spawn":
        entity.pop("brain", None)
        entity.pop("avatar", None)
    return entity


def list_archetype_labels() -> list[tuple[str, str]]:
    return [(k, v.label) for k, v in ARCHETYPES.items()]
