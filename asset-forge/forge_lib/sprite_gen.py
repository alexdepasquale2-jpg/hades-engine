from __future__ import annotations

import hashlib
import math
import random
from dataclasses import asdict, dataclass, field
from typing import Any

from PIL import Image, ImageDraw, ImageFilter, ImageFont

SHAPES = ("rounded_rect", "circle", "diamond", "hex", "shield")
PATTERNS = ("solid", "stripes", "noise", "rim", "pulse")
GLYPHS = ("letter", "none", "star", "bolt", "heart", "skull")


@dataclass
class SpriteSpec:
    version: int = 1
    seed: int = 1
    size: int = 64
    shape: str = "rounded_rect"
    pattern: str = "rim"
    glyph: str = "letter"
    primary: str = "#4872ba"
    accent: str = "#e8eef8"
    glow_strength: float = 0.35
    outline: bool = True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SpriteSpec:
        known = {f.name for f in cls.__dataclass_fields__.values()}
        return cls(**{k: v for k, v in data.items() if k in known})


def _hex_to_rgb(hex_color: str) -> tuple[int, int, int]:
    h = hex_color.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def _rng(seed: int) -> random.Random:
    return random.Random(seed)


def random_sprite_spec(
    seed: int,
    kind: str,
    *,
    size: int = 64,
) -> SpriteSpec:
    r = _rng(seed)
    palettes = {
        "npc": [("#4872ba", "#dce6f5"), ("#6b4e9e", "#efe8ff"), ("#2d7a6b", "#d4f5ee")],
        "prop": [("#785a48", "#f0e4d8"), ("#5a6078", "#e2e6f0")],
        "pickup": [("#b8943a", "#fff4d6"), ("#c45c8a", "#ffe8f2")],
        "player_spawn": [("#3da86e", "#e0f8ea")],
    }
    primary, accent = r.choice(palettes.get(kind, palettes["npc"]))
    return SpriteSpec(
        seed=seed,
        size=size,
        shape=r.choice(SHAPES),
        pattern=r.choice(PATTERNS),
        glyph=r.choice(GLYPHS if kind != "prop" else ("none", "star", "bolt")),
        primary=primary,
        accent=accent,
        glow_strength=round(r.uniform(0.15, 0.55), 2),
        outline=r.choice([True, True, False]),
    )


def _font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    try:
        return ImageFont.truetype("arial.ttf", size)
    except OSError:
        return ImageFont.load_default()


def _shape_points(shape: str, size: int, margin: int) -> list[tuple[float, float]]:
    cx, cy = size / 2, size / 2
    r = size / 2 - margin
    if shape == "circle":
        return [(cx, cy, r)]
    if shape == "diamond":
        return [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]
    if shape == "hex":
        pts = []
        for i in range(6):
            ang = math.pi / 3 * i - math.pi / 6
            pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
        return pts
    if shape == "shield":
        return [
            (cx, cy - r),
            (cx + r * 0.85, cy - r * 0.2),
            (cx + r * 0.7, cy + r * 0.85),
            (cx, cy + r),
            (cx - r * 0.7, cy + r * 0.85),
            (cx - r * 0.85, cy - r * 0.2),
        ]
    m = margin
    return [(m, m), (size - m, m), (size - m, size - m), (m, size - m)]


def _draw_glyph(draw: ImageDraw.ImageDraw, glyph: str, letter: str, size: int, color: tuple[int, int, int, int]) -> None:
    cx, cy = size // 2, size // 2
    if glyph == "none":
        return
    if glyph == "letter":
        font = _font(max(12, size // 2))
        ch = (letter[:1] or "?").upper()
        bbox = draw.textbbox((0, 0), ch, font=font)
        tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
        draw.text((cx - tw / 2, cy - th / 2 - 1), ch, fill=color, font=font)
        return
    s = size // 5
    fill = color
    if glyph == "star":
        pts = []
        for i in range(10):
            ang = math.pi / 5 * i - math.pi / 2
            rad = s * 1.4 if i % 2 == 0 else s * 0.55
            pts.append((cx + rad * math.cos(ang), cy + rad * math.sin(ang)))
        draw.polygon(pts, fill=fill)
    elif glyph == "heart":
        draw.ellipse([cx - s, cy - s, cx, cy + s // 2], fill=fill)
        draw.ellipse([cx, cy - s, cx + s, cy + s // 2], fill=fill)
        draw.polygon([(cx - s, cy), (cx + s, cy), cx, cy + s + 2], fill=fill)
    elif glyph == "bolt":
        draw.polygon(
            [(cx - s // 2, cy - s), (cx + s // 3, cy - s // 3), (cx - s // 4, cy), (cx + s // 2, cy + s)],
            fill=fill,
        )
    elif glyph == "skull":
        draw.ellipse([cx - s, cy - s, cx + s, cy + s], fill=fill)
        eye = s // 3
        draw.ellipse([cx - s // 2 - eye, cy - eye, cx - s // 2 + eye, cy + eye], fill=(30, 30, 40, 220))
        draw.ellipse([cx + s // 2 - eye, cy - eye, cx + s // 2 + eye, cy + eye], fill=(30, 30, 40, 220))


def render_sprite(spec: SpriteSpec, display_name: str) -> Image.Image:
    size = int(spec.size)
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    primary = _hex_to_rgb(spec.primary)
    accent = _hex_to_rgb(spec.accent)
    margin = max(2, size // 12)

    if spec.glow_strength > 0:
        glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        gdraw = ImageDraw.Draw(glow)
        cx, cy = size / 2, size / 2
        gr = size / 2
        gdraw.ellipse(
            [cx - gr, cy - gr, cx + gr, cy + gr],
            fill=primary + (int(80 * spec.glow_strength),),
        )
        glow = glow.filter(ImageFilter.GaussianBlur(radius=max(2, size // 16)))
        img = Image.alpha_composite(img, glow)

    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    shape = spec.shape if spec.shape in SHAPES else "rounded_rect"
    pts = _shape_points(shape, size, margin)

    if shape == "circle" and len(pts) == 1:
        cx, cy, r = pts[0]
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=primary + (255,))
    elif shape == "rounded_rect":
        draw.rounded_rectangle(
            [margin, margin, size - margin, size - margin],
            radius=size // 8,
            fill=primary + (255,),
        )
    else:
        draw.polygon([(p[0], p[1]) for p in pts], fill=primary + (255,))

    if spec.pattern == "stripes":
        r = _rng(spec.seed)
        for i in range(0, size, max(4, size // 8)):
            draw.line([(i, 0), (i - size, size)], fill=accent + (90,), width=max(1, size // 32))
    elif spec.pattern == "noise":
        r = _rng(spec.seed + 7)
        for _ in range(size * 2):
            x, y = r.randint(margin, size - margin), r.randint(margin, size - margin)
            draw.point((x, y), fill=accent + (r.randint(40, 120),))
    elif spec.pattern == "rim":
        if shape == "circle" and pts:
            cx, cy, rad = pts[0]
            draw.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], outline=accent + (200,), width=max(2, size // 24))
        else:
            draw.polygon([(p[0], p[1]) for p in pts], outline=accent + (200,))
    elif spec.pattern == "pulse":
        cx, cy = size / 2, size / 2
        for ring, alpha in [(0.75, 60), (0.55, 90)]:
            rad = (size / 2 - margin) * ring
            draw.ellipse(
                [cx - rad, cy - rad, cx + rad, cy + rad],
                outline=accent + (alpha,),
                width=max(1, size // 28),
            )

    if spec.outline:
        if shape == "circle" and pts:
            cx, cy, rad = pts[0]
            draw.ellipse(
                [cx - rad, cy - rad, cx + rad, cy + rad],
                outline=(20, 24, 32, 180),
                width=max(1, size // 32),
            )

    _draw_glyph(draw, spec.glyph, display_name, size, accent + (240,))
    img = Image.alpha_composite(img, layer)
    return img


def spec_from_entity(entity: dict[str, Any]) -> SpriteSpec | None:
    raw = entity.get("sprite_gen")
    if not raw:
        return None
    return SpriteSpec.from_dict(raw)


def stable_seed(entity_id: str, extra: int = 0) -> int:
    digest = hashlib.sha256(f"{entity_id}:{extra}".encode()).digest()
    return int.from_bytes(digest[:4], "big")
