from __future__ import annotations

import streamlit as st

from mech_lib.diff import diff_rulesets
from mech_lib.ruleset_io import list_ruleset_ids, load_ruleset_by_id


def render_compare() -> None:
    st.markdown("### Ruleset diff")
    ids = list_ruleset_ids()
    c1, c2, c3 = st.columns(3)
    left_id = c1.selectbox("Left (A)", ids, key="cmp_left")
    right_id = c2.selectbox("Right (B)", ids, index=min(1, len(ids) - 1), key="cmp_right")
    use_draft = c3.checkbox("Right = current draft", value=True)

    a = load_ruleset_by_id(left_id)
    b = st.session_state.draft if use_draft else load_ruleset_by_id(right_id)
    rows = diff_rulesets(a, b)
    st.caption(f"{len(rows)} differing path(s)")
    if rows:
        st.dataframe(rows, use_container_width=True, hide_index=True)
    else:
        st.success("Identical under flattened comparison.")
