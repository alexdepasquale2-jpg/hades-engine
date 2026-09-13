from __future__ import annotations

from typing import Any

import streamlit as st

from forge_lib.packs import list_pack_ids, list_ruleset_ids, load_manifest


def init_project_state() -> None:
    if "studio_workspace" not in st.session_state:
        st.session_state.studio_workspace = "scene"
    packs = list_pack_ids()
    if "studio_pack" not in st.session_state:
        st.session_state.studio_pack = packs[0] if packs else None
    if "studio_ruleset" not in st.session_state:
        st.session_state.studio_ruleset = _pack_ruleset(st.session_state.studio_pack) or (
            list_ruleset_ids()[0] if list_ruleset_ids() else "pmr.v1"
        )
    if "studio_selected_entity" not in st.session_state:
        st.session_state.studio_selected_entity = None


def _pack_ruleset(pack_id: str | None) -> str | None:
    if not pack_id:
        return None
    try:
        m = load_manifest(pack_id)
        return m.get("ruleset_id")
    except OSError:
        return None


def sync_ruleset_from_pack() -> None:
    rid = _pack_ruleset(st.session_state.studio_pack)
    if rid:
        st.session_state.studio_ruleset = rid


def project_summary() -> dict[str, Any]:
    return {
        "pack": st.session_state.studio_pack,
        "ruleset": st.session_state.studio_ruleset,
    }
