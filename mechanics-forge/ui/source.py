from __future__ import annotations

import json

import streamlit as st

from mech_lib import history
from mech_lib.editor import apply_json_merge
from mech_lib.ruleset_io import validate_ruleset


def render_source() -> None:
    st.markdown("### JSON source")
    st.caption("Power-user mode: edit raw ruleset JSON, then apply to the draft.")

    draft = st.session_state.draft
    raw = st.text_area(
        "Ruleset JSON",
        value=json.dumps(draft, indent=2),
        height=480,
        key="json_source_area",
    )

    c1, c2, c3 = st.columns(3)
    if c1.button("Apply JSON to draft", type="primary"):
        merged, err = apply_json_merge(draft, raw)
        if err:
            st.error(err)
        else:
            st.session_state.draft = merged
            history.push(merged)
            st.toast("JSON applied")
            st.rerun()
    if c2.button("Reset editor from draft"):
        st.rerun()
    if c3.button("Pretty-print draft"):
        st.session_state.json_source_override = json.dumps(draft, indent=2)
        st.rerun()

    errs = validate_ruleset(st.session_state.draft)
    if errs:
        st.warning("\n".join(errs))
    else:
        st.success("Draft is structurally valid.")
