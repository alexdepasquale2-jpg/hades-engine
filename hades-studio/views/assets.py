from __future__ import annotations

import streamlit as st

from forge_lib.builder import build_entity_from_controls, draft_defaults, preview_spec_from_entity
from forge_lib.entities import list_entities_in_pack, load_entity, write_entity
from forge_lib.packs import create_pack, list_pack_ids, load_manifest, register_entity_in_manifest, save_manifest
from forge_lib.sprite_gen import GLYPHS, PATTERNS, SHAPES, render_sprite
from forge_lib.sprites import write_entity_sprite
from forge_lib.templates import list_archetype_labels
from forge_lib.validate import validate_pack


def _ensure_asset_draft() -> None:
    if "asset_draft" not in st.session_state:
        st.session_state.asset_draft = draft_defaults()
    if "asset_seed" not in st.session_state:
        st.session_state.asset_seed = st.session_state.asset_draft["sprite_gen"]["seed"]


def render_assets() -> None:
    _ensure_asset_draft()
    pack_id = st.session_state.studio_pack
    tab_studio, tab_pack, tab_validate = st.tabs(["Asset studio", "Pack & ruleset", "Validate"])

    with tab_studio:
        draft = st.session_state.asset_draft
        labels = list_archetype_labels()
        arch_keys = [k for k, _ in labels]
        arch_labels = {k: lab for k, lab in labels}
        left, right = st.columns([1.2, 1])
        with left:
            arch_pick = st.selectbox(
                "Archetype",
                arch_keys,
                format_func=lambda k: arch_labels[k],
                key="studio_arch",
            )
            eid = st.text_input("Entity id", value=draft.get("id", "npc.new"))
            name = st.text_input("Name", value=draft.get("display_name", "New"))
            sg = draft.get("sprite_gen", {})
            shape = st.selectbox(
                "Shape",
                SHAPES,
                index=SHAPES.index(sg.get("shape", "circle"))
                if sg.get("shape") in SHAPES
                else 0,
            )
            pattern = st.selectbox("Pattern", PATTERNS, index=0)
            glyph = st.selectbox("Glyph", GLYPHS, index=0)
            seed = st.number_input("Art seed", value=int(st.session_state.asset_seed), step=1)
            st.session_state.asset_seed = seed
        with right:
            spec = preview_spec_from_entity(draft)
            img = render_sprite(spec, name)
            st.image(img, caption="Live preview", width=200)
        if st.button("Build draft from controls"):
            st.session_state.asset_draft = build_entity_from_controls(
                archetype=arch_pick,
                entity_id=eid,
                display_name=name,
                lore=draft.get("lore", ""),
                tags_csv=",".join(draft.get("tags", [])),
                kind=draft.get("kind", "npc"),
                interactable=draft.get("interactable", True),
                hp=float(draft.get("avatar", {}).get("hp", 100)),
                stamina=float(draft.get("avatar", {}).get("stamina", 80)),
                brain_kind=draft.get("brain", {}).get("kind", "idle"),
                brain_seed=int(draft.get("brain", {}).get("seed", 1)),
                pos_x=0.0,
                pos_y=0.0,
                pos_z=0.0,
                rotation=0.0,
                shape=shape,
                pattern=pattern,
                glyph=glyph,
                primary=sg.get("primary", "#4488cc"),
                accent=sg.get("accent", "#ffcc66"),
                glow=float(sg.get("glow", 0.3)),
                sprite_size=int(sg.get("size", 64)),
                outline=bool(sg.get("outline", True)),
                art_seed=int(seed),
            )
            st.rerun()
        if pack_id and st.button("Export to active pack", type="primary"):
            ent = st.session_state.asset_draft
            slug = ent["id"].split(".")[-1]
            rel = f"entities/{slug}.json"
            register_entity_in_manifest(pack_id, rel)
            write_entity(pack_id, ent)
            write_entity_sprite(pack_id, ent)
            st.success(f"Wrote `{ent['id']}` into `{pack_id}`.")

    with tab_pack:
        new_id = st.text_input("New pack id", value="pack.mygame")
        new_title = st.text_input("Title", value="My game slice")
        rs = st.text_input("Ruleset id", value=st.session_state.studio_ruleset)
        if st.button("Create pack"):
            create_pack(new_id, new_title, rs)
            st.session_state.studio_pack = new_id
            st.session_state.studio_ruleset = rs
            st.rerun()
        if pack_id:
            if st.button("Set pack ruleset to project ruleset"):
                m = load_manifest(pack_id)
                m["ruleset_id"] = st.session_state.studio_ruleset
                save_manifest(pack_id, m)
                st.success("Manifest updated.")
        st.caption(f"Active pack: `{pack_id or 'none'}` · packs: {', '.join(list_pack_ids()) or 'none'}")

    with tab_validate:
        if not pack_id:
            st.info("Select a pack first.")
            return
        errs = validate_pack(pack_id)
        if errs:
            for e in errs:
                st.error(e)
        else:
            st.success("Pack validates.")
