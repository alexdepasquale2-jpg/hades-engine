from __future__ import annotations

import streamlit as st

from mech_lib import history
from mech_lib.ruleset_io import load_ruleset_by_id
from ui.catalog import render_catalog
from ui.compare import render_compare
from ui.extensions import render_extensions
from ui.sidebar import render_sidebar
from ui.source import render_source
from ui.workbench import render_workbench


def _init_mechanics() -> None:
    history.init()
    if "draft" not in st.session_state:
        rid = st.session_state.studio_ruleset
        try:
            loaded = load_ruleset_by_id(rid)
        except OSError:
            loaded = {"id": "pmr.custom.v1", "title": "Custom"}
        st.session_state.draft = loaded
        history.set_baseline(loaded)


def render_mechanics() -> None:
    _init_mechanics()
    st.caption(f"Project ruleset: `{st.session_state.studio_ruleset}`")
    if st.button("Load project ruleset into draft"):
        loaded = load_ruleset_by_id(st.session_state.studio_ruleset)
        st.session_state.draft = loaded
        history.set_baseline(loaded)
        st.rerun()

    rail, main = st.columns([1, 3.2])
    with rail:
        with st.container(border=True):
            page = render_sidebar(rail)
    with main:
        if page == "workbench":
            render_workbench()
        elif page == "compare":
            render_compare()
        elif page == "catalog":
            render_catalog()
        elif page == "extensions":
            render_extensions()
        elif page == "source":
            render_source()
        else:
            render_workbench()
