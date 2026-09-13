from __future__ import annotations

import json

import streamlit as st

from mech_lib import history
from mech_lib.extensions import (
    list_extensions,
    load_extension,
    new_extension,
    save_extension,
    validate_extension,
)
from mech_lib.patch_paths import PATCH_PATH_CATALOG, describe_path
from mech_lib.paths import EXTENSIONS_DIR
from mech_lib.ruleset_io import apply_patches


def render_extensions() -> None:
    st.markdown("### Custom mechanic extensions")
    st.caption(f"Stored in `{EXTENSIONS_DIR}`")

    ext_paths = list_extensions()
    if ext_paths:
        pick = st.selectbox("Load", ext_paths, format_func=lambda p: p.stem, key="ext_load")
        if st.button("Load into editor"):
            st.session_state.ext_draft = load_extension(pick)
            st.rerun()

    if "ext_draft" not in st.session_state:
        st.session_state.ext_draft = new_extension("my_mechanic", "My Mechanic")

    ext = st.session_state.ext_draft
    c1, c2 = st.columns(2)
    ext["id"] = c1.text_input("Id", value=ext.get("id", "custom.my"), key="ext_id")
    ext["title"] = c2.text_input("Title", value=ext.get("title", ""), key="ext_title")
    ext["description"] = st.text_area("Description", value=ext.get("description", ""), key="ext_desc")
    ext["engine_status"] = st.selectbox(
        "Status",
        ["ruleset_patch", "design_only", "planned"],
        index=["ruleset_patch", "design_only", "planned"].index(ext.get("engine_status", "ruleset_patch")),
        key="ext_status",
    )

    st.markdown("#### Patches")
    paths = [p for p, _ in PATCH_PATH_CATALOG]
    with st.expander("Path reference"):
        st.dataframe(
            [{"path": p, "hint": d} for p, d in PATCH_PATH_CATALOG],
            hide_index=True,
            use_container_width=True,
            height=200,
        )

    patches = ext.setdefault("patches", {})
    if "patch_table" not in st.session_state or st.button("Sync table from extension"):
        st.session_state.patch_table = [
            {"path": k, "value_json": json.dumps(v)}
            for k, v in patches.items()
        ] or [{"path": "verbs.attack.damage", "value_json": "40"}]

    edited = st.data_editor(
        st.session_state.patch_table,
        num_rows="dynamic",
        use_container_width=True,
        column_config={
            "path": st.column_config.SelectboxColumn("path", options=paths, required=True),
            "value_json": st.column_config.TextColumn("value (JSON)", required=True),
        },
        key="ext_patch_editor",
    )
    st.session_state.patch_table = edited

    new_patches: dict = {}
    rows = edited.to_dict("records") if hasattr(edited, "to_dict") else edited
    for row in rows:
        path = (row.get("path") or "").strip()
        raw = row.get("value_json") or ""
        if not path:
            continue
        try:
            new_patches[path] = json.loads(raw)
        except json.JSONDecodeError:
            new_patches[path] = raw
    ext["patches"] = new_patches

    if new_patches:
        sample = next(iter(new_patches))
        st.caption(describe_path(sample))

    st.session_state.ext_draft = ext

    b1, b2, b3 = st.columns(3)
    if b1.button("Save extension", type="primary"):
        errs = validate_extension(ext)
        if errs:
            st.error("\n".join(errs))
        else:
            st.success(f"Saved {save_extension(ext)}")
    if b2.button("Apply to draft"):
        st.session_state.draft = apply_patches(st.session_state.draft, new_patches)
        history.push(st.session_state.draft)
        st.toast("Patches merged into ruleset draft")
    if b3.button("New extension"):
        st.session_state.ext_draft = new_extension("mechanic", "New Mechanic")
        st.session_state.pop("patch_table", None)
        st.rerun()
