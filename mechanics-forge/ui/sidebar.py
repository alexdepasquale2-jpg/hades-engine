from __future__ import annotations

import json
from copy import deepcopy

import streamlit as st

from mech_lib import history
from mech_lib.diff import count_diff
from mech_lib.presets import PRESETS, preset_names
from mech_lib.ruleset_io import (
    apply_patches,
    blank_from_template,
    list_ruleset_ids,
    load_ruleset_by_id,
    save_ruleset,
    validate_ruleset,
)


def render_sidebar() -> str:
    """Returns selected workspace page id."""
    st.markdown("### Mechanics Forge")
    ids = list_ruleset_ids()
    draft = st.session_state.draft

    dirty = history.is_dirty(draft)
    chip = "forge-chip-dirty" if dirty else "forge-chip-clean"
    label = "Unsaved changes" if dirty else "Synced with baseline"
    st.markdown(f'<span class="forge-chip {chip}">{label}</span>', unsafe_allow_html=True)

    st.markdown("#### Ruleset file")
    pick = st.selectbox("Open", ids or ["pmr.v1"], key="sidebar_open", label_visibility="collapsed")
    c1, c2 = st.columns(2)
    if c1.button("Load", use_container_width=True):
        loaded = load_ruleset_by_id(pick)
        st.session_state.draft = loaded
        history.set_baseline(loaded)
        st.session_state.ui_section = "Core"
        st.rerun()
    if c2.button("Save", type="primary", use_container_width=True):
        errs = validate_ruleset(draft)
        if errs:
            st.session_state.save_errors = errs
        else:
            save_ruleset(draft)
            history.set_baseline(draft)
            st.session_state.save_ok = draft.get("id")
            st.rerun()

    u1, u2, u3 = st.columns(3)
    if u1.button("Undo", disabled=not history.can_undo(), use_container_width=True):
        prev = history.undo()
        if prev is not None:
            st.session_state.draft = prev
            st.rerun()
    if u2.button("Redo", disabled=not history.can_redo(), use_container_width=True):
        nxt = history.redo()
        if nxt is not None:
            st.session_state.draft = nxt
            st.rerun()
    if u3.button("Revert", use_container_width=True):
        base = st.session_state.baseline_draft
        if base:
            st.session_state.draft = deepcopy(base)
            history.push(st.session_state.draft)
            st.rerun()

    if st.session_state.get("save_ok"):
        st.success(f"Saved `{st.session_state.pop('save_ok')}`")
    if st.session_state.get("save_errors"):
        for e in st.session_state.pop("save_errors"):
            st.error(e)

    with st.expander("New from template"):
        base = st.selectbox("Template", ids or ["pmr.v1"], key="sidebar_tpl")
        new_id = st.text_input("Id", value="pmr.custom.v1", key="sidebar_new_id")
        new_title = st.text_input("Title", value="Custom ruleset", key="sidebar_new_title")
        if st.button("Create draft"):
            created = blank_from_template(base, new_id, new_title)
            st.session_state.draft = created
            history.set_baseline(created)
            st.rerun()

    preset = st.selectbox("Quick preset patch", ["—"] + preset_names(), key="sidebar_preset")
    if preset != "—" and st.button("Apply preset to draft"):
        st.session_state.draft = apply_patches(draft, PRESETS[preset])
        history.push(st.session_state.draft)
        st.rerun()

    base = st.session_state.baseline_draft
    if base:
        n = count_diff(base, draft)
        st.caption(f"{n} field(s) differ from baseline")

    st.divider()
    _pages = {
        "workbench": "Workbench",
        "compare": "Compare",
        "catalog": "Catalog",
        "extensions": "Extensions",
        "source": "JSON source",
    }
    return st.radio(
        "Workspace",
        list(_pages.keys()),
        format_func=lambda k: _pages[k],
        key="sidebar_page",
        label_visibility="collapsed",
    )
