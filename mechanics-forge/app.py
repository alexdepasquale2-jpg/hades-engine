"""Mechanics Forge — professional ruleset & mechanics workbench."""

from __future__ import annotations

import sys
from pathlib import Path

import streamlit as st

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from mech_lib import history
from mech_lib.ruleset_io import list_ruleset_ids, load_ruleset_by_id
from mech_lib.styles import FORGE_CSS
from ui.catalog import render_catalog
from ui.compare import render_compare
from ui.extensions import render_extensions
from ui.sidebar import render_sidebar
from ui.source import render_source
from ui.workbench import render_workbench

st.set_page_config(
    page_title="Mechanics Forge",
    page_icon="⚙",
    layout="wide",
    initial_sidebar_state="expanded",
)

st.markdown(FORGE_CSS, unsafe_allow_html=True)

history.init()
if "draft" not in st.session_state:
    ids = list_ruleset_ids()
    base = ids[0] if ids else "pmr.v1"
    try:
        loaded = load_ruleset_by_id(base)
    except OSError:
        loaded = {"id": "pmr.custom.v1", "title": "Custom"}
    st.session_state.draft = loaded
    history.set_baseline(loaded)

with st.sidebar:
    page = render_sidebar()

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
