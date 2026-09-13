from __future__ import annotations

import random
import re
from typing import Any

from forge_lib.sprite_gen import SpriteSpec, random_sprite_spec, stable_seed
from forge_lib.templates import ARCHETYPES, apply_archetype

_SYLLABLES_A = ("ver", "ash", "mor", "keth", "lun", "dra", "sil", "tor", "fen", "ria")
_SYLLABLES_B = ("on", "is", "el", "ar", "ix", "um", "eth", "or", "an", "us")


def random_display_name(rng: random.Random) -> str:
    a, b = rng.choice(_SYLLABLES_A), rng.choice(_SYLLABLES_B)
    name = (a + b).capitalize()
    if rng.random() < 0.35:
        name += f" {rng.choice(('the Bold', 'of Echo', 'Shard-Keeper', 'Warden'))}"
    return name


def slugify(name: str) -> str:
    base = re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
    return base[:24] or "entity"


def random_entity_id(kind: str, rng: random.Random, display_name: str) -> str:
    prefix = {"npc": "npc", "prop": "prop", "pickup": "pickup", "player_spawn": "spawn"}.get(
        kind, "npc"
    )
    return f"{prefix}.{slugify(display_name)}"


def generate_random_asset(
    pack_ruleset: str = "pmr.v1",
    *,
    seed: int | None = None,
) -> dict[str, Any]:
    rng = random.Random(seed if seed is not None else random.randint(0, 2**31))
    archetype_key = rng.choice(list(ARCHETYPES.keys()))
    arch = ARCHETYPES[archetype_key]
    name = random_display_name(rng)
    eid = random_entity_id(arch.kind, rng, name)
    ent_seed = stable_seed(eid, rng.randint(0, 9999))
    x, y = round(rng.uniform(-48, 48), 1), round(rng.uniform(-48, 48), 1)
    entity = apply_archetype(
        archetype_key,
        eid,
        name,
        position={"x": x, "y": y, "z": 0.0},
        rotation=round(rng.uniform(0, 6.28), 2),
        sprite_seed=ent_seed,
    )
    blurb = (
        f"A {arch.label.lower()} suited for `{pack_ruleset}` — "
        f"{arch.description} Generated seed {ent_seed}."
    )
    entity["lore"] = blurb
    return entity


def merge_sprite_controls(
    entity: dict[str, Any],
    *,
    shape: str,
    pattern: str,
    glyph: str,
    primary: str,
    accent: str,
    glow: float,
    size: int,
    outline: bool,
    seed: int,
) -> dict[str, Any]:
    spec = SpriteSpec(
        seed=seed,
        size=size,
        shape=shape,
        pattern=pattern,
        glyph=glyph,
        primary=primary,
        accent=accent,
        glow_strength=glow,
        outline=outline,
    )
    entity = dict(entity)
    entity["sprite"] = {
        "path": entity.get("sprite", {}).get("path", f"sprites/{entity['id'].split('.')[-1]}.png"),
        "size": size,
    }
    entity["sprite_gen"] = spec.to_dict()
    return entity
