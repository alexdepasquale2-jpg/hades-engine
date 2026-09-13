from __future__ import annotations

from pathlib import Path

import streamlit as st

from forge_lib.entities import list_entities_in_pack, load_entity, write_entity
from forge_lib.map_preview import render_pack_map
from forge_lib.packs import load_manifest, save_manifest, upsert_placement
from forge_lib.paths import PACKS_DIR


def render_scene() -> None:
    pack_id = st.session_state.studio_pack
    if not pack_id:
        st.warning("Create or select a content pack under **Assets → Pack & ruleset**.")
        return

    manifest = load_manifest(pack_id)
    entity_paths = list_entities_in_pack(pack_id)

    col_hier, col_view, col_insp = st.columns([1, 2, 1.2])

    with col_hier:
        st.markdown('<p class="studio-panel-label">Hierarchy</p>', unsafe_allow_html=True)
        st.caption(f"Pack `{pack_id}`")
        st.caption(f"Ruleset `{manifest.get('ruleset_id', '?')}`")
        for ent_path in entity_paths:
            ent = load_entity(ent_path)
            label = ent.get("display_name") or ent.get("id", ent_path.name)
            if st.button(label, key=f"hier_{ent_path.name}", use_container_width=True):
                st.session_state.studio_selected_entity = ent_path.name
                st.rerun()

    with col_view:
        st.markdown('<p class="studio-panel-label">Scene view</p>', unsafe_allow_html=True)
        placements = manifest.get("placements") or []
        png = render_pack_map(pack_id, placements=placements)
        st.image(png, use_container_width=True)
        st.caption("Top-down placement map (world units).")

    with col_insp:
        st.markdown('<p class="studio-panel-label">Inspector</p>', unsafe_allow_html=True)
        sel_name = st.session_state.studio_selected_entity
        ent_path = next((p for p in entity_paths if p.name == sel_name), None)
        if not ent_path:
            st.info("Select an entity in the hierarchy.")
            return
        entity = load_entity(ent_path)
        st.text_input("Id", value=entity.get("id", ""), disabled=True)
        name = st.text_input("Display name", value=entity.get("display_name", ""))
        kind = st.text_input("Kind", value=entity.get("kind", "npc"))
        pos = entity.get("transform_default", {}).get("position", {})
        px = st.number_input("X", value=float(pos.get("x", 0)), format="%.1f")
        py = st.number_input("Y", value=float(pos.get("y", 0)), format="%.1f")
        pz = st.number_input("Z", value=float(pos.get("z", 0)), format="%.1f")
        if st.button("Apply transform", type="primary"):
            entity["display_name"] = name
            entity["kind"] = kind
            entity.setdefault("transform_default", {})["position"] = {
                "x": px,
                "y": py,
                "z": pz,
            }
            write_entity(pack_id, entity)
            rel = f"entities/{ent_path.name}"
            upsert_placement(pack_id, rel, px, py, pz)
            st.success("Saved entity & placement.")
            st.rerun()
