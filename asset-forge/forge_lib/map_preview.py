from __future__ import annotations

import io
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw, ImageFont

from forge_lib.entities import load_entity
from forge_lib.paths import PACKS_DIR
from forge_lib.sprite_gen import render_sprite, spec_from_entity
from forge_lib.sprites import render_placeholder_sprite


def _load_thumbnail(entity_path: Path, size: int = 24) -> Image.Image:
    entity = load_entity(entity_path)
    spec = spec_from_entity(entity)
    if spec:
        img = render_sprite(spec, entity.get("display_name", ""))
    else:
        img = render_placeholder_sprite(
            entity.get("display_name", ""),
            entity.get("kind", "npc"),
            entity.get("id", ""),
            size=32,
        )
    img = img.resize((size, size), Image.Resampling.LANCZOS)
    return img


def render_pack_map(
    pack_id: str,
    placements: list[dict[str, Any]] | None = None,
    canvas: int = 400,
    world_extent: float = 96.0,
) -> bytes:
    pack_dir = PACKS_DIR / pack_id
    entities_dir = pack_dir / "entities"
    img = Image.new("RGBA", (canvas, canvas), (18, 22, 30, 255))
    draw = ImageDraw.Draw(img)
    step = canvas // 8
    for i in range(0, canvas, step):
        draw.line([(i, 0), (i, canvas)], fill=(40, 48, 62, 255))
        draw.line([(0, i), (canvas, i)], fill=(40, 48, 62, 255))
    cx, cy = canvas // 2, canvas // 2
    draw.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], fill=(120, 140, 180, 200))

    placement_by_rel: dict[str, dict[str, float]] = {}
    if placements:
        for p in placements:
            placement_by_rel[p.get("entity", "")] = {
                "x": float(p.get("x", 0)),
                "y": float(p.get("y", 0)),
            }

    for ent_path in sorted(entities_dir.glob("*.json")):
        entity = load_entity(ent_path)
        rel = f"entities/{ent_path.name}"
        pos = entity.get("transform_default", {}).get("position", {})
        if rel in placement_by_rel:
            pos = {**pos, **placement_by_rel[rel]}
        wx, wy = float(pos.get("x", 0)), float(pos.get("y", 0))
        px = cx + int(wx / world_extent * (canvas / 2 - 20))
        py = cy - int(wy / world_extent * (canvas / 2 - 20))
        thumb = _load_thumbnail(ent_path)
        img.paste(thumb, (px - thumb.width // 2, py - thumb.height // 2), thumb)
        try:
            font = ImageFont.truetype("arial.ttf", 9)
        except OSError:
            font = ImageFont.load_default()
        label = entity.get("display_name", ent_path.stem)[:10]
        draw.text((px + 14, py - 6), label, fill=(220, 228, 240, 255), font=font)

    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()
