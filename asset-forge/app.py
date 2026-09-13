"""Asset Forge — procedural studio for full in-app asset authoring."""

from __future__ import annotations

import io
import json
import random
import sys
from pathlib import Path

import streamlit as st
from PIL import Image

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from forge_lib.builder import build_entity_from_controls, draft_defaults, preview_spec_from_entity
from forge_lib.entities import list_entities_in_pack, load_entity, write_entity
from forge_lib.generate import generate_random_asset
from forge_lib.map_preview import render_pack_map
from forge_lib.packs import (
    create_pack,
    list_pack_ids,
    list_ruleset_ids,
    load_manifest,
    register_entity_in_manifest,
    upsert_placement,
)
from forge_lib.paths import ASSETS_DIR, PACKS_DIR, RULESETS_DIR
from forge_lib.ruleset import scaffold_ruleset_variant
from forge_lib.sprite_gen import GLYPHS, PATTERNS, SHAPES, render_sprite
from forge_lib.sprites import write_entity_sprite
from forge_lib.templates import ARCHETYPES, list_archetype_labels
from forge_lib.validate import validate_all, validate_pack

st.set_page_config(
    page_title="Asset Forge — Hades Engine",
    page_icon="🛠",
    layout="wide",
)

st.markdown(
    """
    <style>
    .block-container { padding-top: 1.2rem; }
    div[data-testid="stSidebar"] { background: linear-gradient(180deg, #1a2030 0%, #12161f 100%); }
    </style>
    """,
    unsafe_allow_html=True,
)

if "draft" not in st.session_state:
    st.session_state.draft = draft_defaults()
if "art_seed" not in st.session_state:
    st.session_state.art_seed = st.session_state.draft["sprite_gen"]["seed"]

pack_ids = list_pack_ids()
rulesets = list_ruleset_ids()
draft = st.session_state.draft
spec = preview_spec_from_entity(draft)

st.title("Asset Forge Studio")
st.caption("Design entities end-to-end: identity, stats, world position, procedural art, then export to your pack.")

side = st.sidebar
side.header("Workflow")
studio_mode = side.radio(
    "Mode",
    ["Create new", "Edit existing"],
    horizontal=True,
)
target_pack: str | None = None
if pack_ids:
    target_pack = side.selectbox("Target pack", pack_ids)
else:
    side.warning("Create a pack under **Library** tab first.")

tab_studio, tab_library, tab_world, tab_ruleset, tab_validate, tab_help = st.tabs(
    ["Studio", "Library", "World map", "Rulesets", "Validate", "Help"]
)

with tab_studio:
    top_l, top_r = st.columns([1, 1])
    with top_l:
        st.subheader("1 · Identity & role")
        archetype_labels = list_archetype_labels()
        arch_keys = [k for k, _ in archetype_labels]
        arch_ix = arch_keys.index(draft.get("archetype", "mentor")) if draft.get("archetype") in arch_keys else 0

        c1, c2, c3 = st.columns(3)
        if c1.button("Randomize all", type="primary"):
            rs = load_manifest(target_pack)["ruleset_id"] if target_pack else "pmr.v1"
            st.session_state.draft = generate_random_asset(rs, seed=random.randint(0, 2**31))
            st.session_state.art_seed = st.session_state.draft["sprite_gen"]["seed"]
            st.rerun()
        if c2.button("Reroll art"):
            st.session_state.art_seed = random.randint(0, 2**31 - 1)
            st.rerun()
        if c3.button("Reset draft"):
            st.session_state.draft = draft_defaults()
            st.session_state.art_seed = st.session_state.draft["sprite_gen"]["seed"]
            st.rerun()

        archetype = st.selectbox(
            "Archetype template",
            arch_keys,
            index=arch_ix,
            format_func=lambda k: ARCHETYPES[k].label,
        )
        display_name = st.text_input("Display name", value=draft.get("display_name", "Entity"))
        entity_id = st.text_input("Entity id", value=draft.get("id", "npc.entity"))
        lore = st.text_area("Lore / description", value=draft.get("lore", ""), height=80)
        tags_csv = st.text_input("Tags (comma-separated)", value=", ".join(draft.get("tags", [])))

        st.subheader("2 · Simulation")
        kind = st.selectbox(
            "Kind",
            ["npc", "prop", "pickup", "player_spawn"],
            index=["npc", "prop", "pickup", "player_spawn"].index(draft.get("kind", "npc")),
        )
        interactable = st.checkbox("Interactable", value=draft.get("interactable", True))
        if kind != "player_spawn":
            hp = st.slider("HP", 1.0, 300.0, float((draft.get("avatar") or {}).get("hp", 100.0)))
            stamina = st.slider(
                "Stamina", 0.0, 200.0, float((draft.get("avatar") or {}).get("stamina", 100.0))
            )
            brain_kind = st.selectbox(
                "Brain",
                ["Utility", "BehaviorTree", "Ml"],
                index=["Utility", "BehaviorTree", "Ml"].index(
                    (draft.get("brain") or {}).get("kind", "Utility")
                ),
            )
            brain_seed = st.number_input(
                "Brain seed",
                min_value=0,
                value=int((draft.get("brain") or {}).get("seed", 1)),
            )
        else:
            hp, stamina, brain_kind, brain_seed = 100.0, 100.0, "Utility", 1

        st.subheader("3 · Placement")
        pos = draft.get("transform_default", {}).get("position", {})
        pos_x = st.slider("World X", -128.0, 128.0, float(pos.get("x", 0.0)))
        pos_y = st.slider("World Y", -128.0, 128.0, float(pos.get("y", 0.0)))
        pos_z = st.slider("World Z", -32.0, 32.0, float(pos.get("z", 0.0)))
        rotation = st.slider(
            "Facing (rad)",
            0.0,
            6.28,
            float(draft.get("transform_default", {}).get("rotation", 0.0)),
        )

        st.subheader("4 · Procedural art")
        sg = draft.get("sprite_gen", {})
        def _ix(opts: tuple[str, ...] | list[str], val: str, default: str) -> int:
            try:
                return opts.index(val)
            except ValueError:
                return opts.index(default)

        shape = st.selectbox(
            "Shape", SHAPES, index=_ix(SHAPES, sg.get("shape", ""), "rounded_rect")
        )
        pattern = st.selectbox(
            "Pattern", PATTERNS, index=_ix(PATTERNS, sg.get("pattern", ""), "rim")
        )
        glyph = st.selectbox(
            "Emblem", GLYPHS, index=_ix(GLYPHS, sg.get("glyph", ""), "letter")
        )
        primary = st.color_picker("Primary", sg.get("primary", "#4872ba"))
        accent = st.color_picker("Accent", sg.get("accent", "#e8eef8"))
        glow = st.slider("Glow", 0.0, 1.0, float(sg.get("glow_strength", 0.35)))
        sprite_size = st.select_slider("Sprite size", [32, 48, 64, 96, 128], value=int(sg.get("size", 64)))
        outline = st.checkbox("Outline", value=bool(sg.get("outline", True)))
        art_seed = st.session_state.art_seed
        st.caption(f"Art seed: `{art_seed}` — reroll with **Reroll art**")

    built = build_entity_from_controls(
        archetype=archetype,
        entity_id=entity_id.strip(),
        display_name=display_name.strip(),
        lore=lore.strip(),
        tags_csv=tags_csv,
        kind=kind,
        interactable=interactable,
        hp=hp,
        stamina=stamina,
        brain_kind=brain_kind,
        brain_seed=int(brain_seed),
        pos_x=pos_x,
        pos_y=pos_y,
        pos_z=pos_z,
        rotation=rotation,
        shape=shape,
        pattern=pattern,
        glyph=glyph,
        primary=primary,
        accent=accent,
        glow=glow,
        sprite_size=int(sprite_size),
        outline=outline,
        art_seed=int(art_seed),
    )
    st.session_state.draft = built

    with top_r:
        st.subheader("Live preview")
        preview_img = render_sprite(preview_spec_from_entity(built), built["display_name"])
        scale = 4 if preview_img.width <= 64 else 2
        w, h = preview_img.width * scale, preview_img.height * scale
        buf = io.BytesIO()
        preview_img.resize((w, h), resample=Image.Resampling.NEAREST).save(buf, format="PNG")
        st.image(buf.getvalue(), caption=f"{built['display_name']} · {built['kind']}")
        st.markdown(
            f"**{built['display_name']}** (`{built['id']}`)  \n"
            f"{built.get('lore', '')}"
        )
        m1, m2, m3 = st.columns(3)
        if kind != "player_spawn":
            m1.metric("HP", f"{built.get('avatar', {}).get('hp', 0):.0f}")
            m2.metric("Stamina", f"{built.get('avatar', {}).get('stamina', 0):.0f}")
        m3.metric("Tags", len(built.get("tags", [])))
        with st.expander("JSON preview", expanded=False):
            st.code(json.dumps(built, indent=2), language="json")

    st.divider()
    save_col, edit_col = st.columns([1, 2])
    with save_col:
        if st.button("Save asset to pack", type="primary", disabled=not target_pack):
            path = write_entity(target_pack, built)
            rel = f"entities/{path.name}"
            register_entity_in_manifest(target_pack, rel)
            upsert_placement(
                target_pack,
                rel,
                float(built["transform_default"]["position"]["x"]),
                float(built["transform_default"]["position"]["y"]),
                float(built["transform_default"]["position"].get("z", 0)),
            )
            sprite_path = write_entity_sprite(target_pack, built)
            errs = validate_pack(target_pack)
            if errs:
                st.warning("Saved with validation notes:\n" + "\n".join(errs[:5]))
            else:
                st.success(f"Saved entity + sprite ({sprite_path.name})")
    with edit_col:
        if studio_mode == "Edit existing" and target_pack:
            ent_files = list_entities_in_pack(target_pack)
            pick = st.selectbox(
                "Load into studio",
                ent_files,
                format_func=lambda p: p.name,
                key="edit_pick",
            )
            if st.button("Load selected entity"):
                st.session_state.draft = load_entity(pick)
                st.session_state.art_seed = st.session_state.draft.get("sprite_gen", {}).get(
                    "seed", 1
                )
                st.rerun()

with tab_library:
    st.subheader("Content packs")
    with st.form("new_pack"):
        pid = st.text_input("Pack id", value="pack.myzone")
        title = st.text_input("Title", value="My zone")
        rs = st.selectbox("Ruleset", rulesets or ["pmr.v1"])
        desc = st.text_area("Description", value="")
        if st.form_submit_button("Create pack"):
            create_pack(pid, title, rs, desc)
            st.success("Pack created.")
            st.rerun()
    if pack_ids:
        selected = st.selectbox("Browse pack", pack_ids, key="lib_pack")
        manifest = load_manifest(selected)
        st.json(manifest)
        pack_dir = PACKS_DIR / selected
        for ent_path in list_entities_in_pack(selected):
            ent = load_entity(ent_path)
            c1, c2, c3 = st.columns([1, 3, 1])
            thumb = render_sprite(
                preview_spec_from_entity(ent), ent.get("display_name", "")
            ).resize((48, 48))
            b = io.BytesIO()
            thumb.save(b, format="PNG")
            c1.image(b.getvalue())
            c2.markdown(f"**{ent['display_name']}** — `{ent['id']}`")
            if c3.button("Open in studio", key=f"open_{ent_path.name}"):
                st.session_state.draft = ent
                st.session_state.art_seed = ent.get("sprite_gen", {}).get("seed", 1)
                st.rerun()

with tab_world:
    st.subheader("Pack world map")
    if not pack_ids:
        st.info("Create a pack to see placements.")
    else:
        wpack = st.selectbox("Pack", pack_ids, key="world_pack")
        manifest = load_manifest(wpack)
        png = render_pack_map(wpack, manifest.get("placements"))
        st.image(png, caption="Top-down placement preview (X right, Y up)")
        st.markdown("Adjust positions in **Studio** sliders, then **Save asset to pack**.")

with tab_ruleset:
    st.caption(f"Rulesets in `{RULESETS_DIR}`")
    with st.form("fork_ruleset"):
        base = st.selectbox("Copy from", rulesets or ["pmr.v1"])
        new_id = st.text_input("New id", value="pmr.custom.v1")
        new_title = st.text_input("New title", value="PMR Custom")
        tightness = st.number_input("Tightness delta", value=0.0, step=0.01, format="%.3f")
        if st.form_submit_button("Write ruleset JSON"):
            try:
                path = scaffold_ruleset_variant(base, new_id, new_title, tightness_delta=tightness)
                st.success(f"Wrote {path}")
            except Exception as exc:
                st.error(str(exc))

with tab_validate:
    if st.button("Validate all", type="primary"):
        errors = validate_all()
        if errors:
            st.error(f"{len(errors)} issue(s)")
            for line in errors:
                st.markdown(f"- `{line}`")
        else:
            st.success("All checks passed.")

with tab_help:
    st.markdown(
        f"""
        ### Asset Studio workflow

        1. Create a pack under **Library**
        2. In **Studio**, pick an archetype or hit **Randomize all**
        3. Tune stats, placement, and procedural art (live preview on the right)
        4. **Save asset to pack** — writes entity JSON, PNG sprite, manifest entry, and map placement

        ### Generative controls

        - **Archetypes** seed stats, tags, and art defaults
        - **Art seed** makes sprites reproducible; **Reroll art** explores variants
        - Shapes, patterns, glyphs, and palettes are rendered procedurally (no external art tools required)

        Output: `{ASSETS_DIR}/packs/...`
        """
    )
