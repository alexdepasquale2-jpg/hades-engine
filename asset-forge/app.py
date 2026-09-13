"""Asset Forge — create game content packs, entities, and placeholder art."""

from __future__ import annotations

import sys
from pathlib import Path

import streamlit as st

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from forge_lib.entities import default_entity, write_entity
from forge_lib.packs import (
    create_pack,
    list_pack_ids,
    list_ruleset_ids,
    load_manifest,
    register_entity_in_manifest,
)
from forge_lib.paths import ASSETS_DIR, PACKS_DIR, RULESETS_DIR
from forge_lib.ruleset import scaffold_ruleset_variant
from forge_lib.sprites import generate_all_sprites
from forge_lib.validate import validate_all, validate_pack

st.set_page_config(
    page_title="Asset Forge — Hades Engine",
    page_icon="🛠",
    layout="wide",
)

st.title("Asset Forge")
st.caption("Create entity defs, content packs, placeholder sprites, and ruleset variants.")

pack_ids = list_pack_ids()
rulesets = list_ruleset_ids()

tab_create, tab_sprites, tab_ruleset, tab_validate, tab_help = st.tabs(
    ["Create content", "Sprites", "Rulesets", "Validate", "Help"]
)

with tab_create:
    st.subheader("New content pack")
    with st.form("new_pack"):
        pid = st.text_input("Pack id", value="pack.myzone", help="Must start with pack.")
        title = st.text_input("Title", value="My zone")
        rs = st.selectbox("Ruleset", rulesets or ["pmr.v1"])
        desc = st.text_area("Description", value="")
        if st.form_submit_button("Create pack"):
            try:
                path = create_pack(pid, title, rs, desc)
                st.success(f"Created {path}")
                st.rerun()
            except Exception as exc:
                st.error(str(exc))

    st.divider()
    st.subheader("New entity")
    if not pack_ids:
        st.info("Create a pack first, or run `python cli.py init-starter` from `asset-forge/`.")
    else:
        with st.form("new_entity"):
            pack = st.selectbox("Pack", pack_ids)
            eid = st.text_input("Entity id", value="npc.guard")
            name = st.text_input("Display name", value="Guard")
            kind = st.selectbox("Kind", ["npc", "prop", "pickup", "player_spawn"])
            hp = st.number_input("HP", min_value=1.0, value=100.0)
            stamina = st.number_input("Stamina", min_value=0.0, value=100.0)
            interact = st.checkbox("Interactable", value=True)
            gen_sprite = st.checkbox("Generate sprites for pack after save", value=True)
            if st.form_submit_button("Add entity"):
                ent = default_entity(
                    eid,
                    name,
                    kind=kind,
                    interactable=interact,
                    hp=hp,
                    stamina=stamina,
                )
                path = write_entity(pack, ent)
                register_entity_in_manifest(pack, f"entities/{path.name}")
                if gen_sprite:
                    generate_all_sprites(pack)
                st.success(f"Wrote {path}")

    if pack_ids:
        st.divider()
        st.subheader("Pack browser")
        selected = st.selectbox("Inspect pack", pack_ids, key="inspect_pack")
        manifest = load_manifest(selected)
        st.json(manifest)
        pack_dir = PACKS_DIR / selected
        sprites = sorted((pack_dir / "sprites").glob("*.png")) if (pack_dir / "sprites").is_dir() else []
        if sprites:
            cols = st.columns(min(6, len(sprites)))
            for i, sp in enumerate(sprites[:12]):
                cols[i % len(cols)].image(str(sp), caption=sp.name, width=64)

with tab_sprites:
    st.subheader("Placeholder sprites")
    st.markdown(
        "Generates 32×32 PNG icons (letter + color by kind) for every entity in a pack. "
        "Swap these files later for real art; keep paths in entity JSON."
    )
    if pack_ids:
        pack = st.selectbox("Pack", pack_ids, key="sprite_pack")
        if st.button("Generate all sprites", type="primary"):
            paths = generate_all_sprites(pack)
            st.success(f"Wrote {len(paths)} file(s).")
            for p in paths:
                st.text(p.relative_to(ASSETS_DIR))
    else:
        st.info("No packs yet.")

with tab_ruleset:
    st.subheader("Fork ruleset JSON")
    st.caption(f"Files live in `{RULESETS_DIR}`")
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
    if st.button("Run validation", type="primary"):
        errors = validate_all()
        if not errors:
            st.success("All checks passed.")
        else:
            st.error(f"{len(errors)} issue(s)")
            for line in errors:
                st.markdown(f"- `{line}`")
    if pack_ids:
        st.divider()
        one = st.selectbox("Validate one pack", pack_ids, key="val_one")
        if st.button("Validate selected pack"):
            errs = validate_pack(one)
            if errs:
                for line in errs:
                    st.warning(line)
            else:
                st.success(f"{one} is valid.")

with tab_help:
    st.markdown(
        f"""
        ### Run locally

        - **Windows:** double-click `run-asset-forge.bat` in the repo root
        - **CLI:** from `asset-forge/`:

        ```powershell
        python cli.py init-starter
        python cli.py validate
        python cli.py new-entity pack.starter npc.scribe Scribe --sprites
        ```

        ### Output layout

        - `{ASSETS_DIR}` — schemas and `packs/<pack_id>/`
        - Entity defs align with engine ECS fields (transform, avatar, brain, interact)
        - Pack `ruleset_id` should match a file in `rulesets/`

        ### Art pipeline

        Placeholder sprites are for prototyping. Replace PNGs under `packs/.../sprites/`
        without changing entity `sprite.path` unless you rename files.
        """
    )
