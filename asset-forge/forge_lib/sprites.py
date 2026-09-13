from __future__ import annotations

import hashlib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

from forge_lib.entities import load_entity
from forge_lib.paths import PACKS_DIR

KIND_COLORS = {
    "npc": (72, 118, 186),
    "prop": (120, 92, 72),
    "pickup": (186, 152, 72),
    "player_spawn": (72, 186, 118),
}


def _color_for(entity_id: str, kind: str) -> tuple[int, int, int]:
    base = KIND_COLORS.get(kind, (100, 100, 120))
    h = hashlib.sha256(entity_id.encode()).digest()
    return (
        min(255, base[0] + h[0] % 40),
        min(255, base[1] + h[1] % 40),
        min(255, base[2] + h[2] % 40),
    )


def render_placeholder_sprite(
    display_name: str,
    kind: str,
    entity_id: str,
    size: int = 32,
) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    fill = _color_for(entity_id, kind)
    margin = max(2, size // 16)
    draw.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=size // 8,
        fill=fill + (255,),
    )
    letter = (display_name[:1] or "?").upper()
    try:
        font = ImageFont.truetype("arial.ttf", max(10, size // 2))
    except OSError:
        font = ImageFont.load_default()
    bbox = draw.textbbox((0, 0), letter, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    draw.text(
        ((size - tw) / 2, (size - th) / 2 - 1),
        letter,
        fill=(255, 255, 255, 230),
        font=font,
    )
    return img


def generate_sprite_for_entity(pack_id: str, entity_path: Path) -> Path:
    entity = load_entity(entity_path)
    sprite = entity.get("sprite") or {}
    rel = sprite.get("path", f"sprites/{entity_path.stem}.png")
    size = int(sprite.get("size", 32))
    pack_dir = PACKS_DIR / pack_id
    out = pack_dir / rel
    out.parent.mkdir(parents=True, exist_ok=True)
    img = render_placeholder_sprite(
        entity.get("display_name", entity_path.stem),
        entity.get("kind", "npc"),
        entity.get("id", entity_path.stem),
        size=size,
    )
    img.save(out)
    return out


def generate_all_sprites(pack_id: str) -> list[Path]:
    from forge_lib.entities import list_entities_in_pack

    written: list[Path] = []
    for entity_path in list_entities_in_pack(pack_id):
        written.append(generate_sprite_for_entity(pack_id, entity_path))
    return written
