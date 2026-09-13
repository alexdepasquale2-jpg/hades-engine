"""Hades Studio — unified game engine editor (scene, assets, mechanics, play, build)."""

from __future__ import annotations

from studio_lib.bootstrap import install_paths

install_paths()

import streamlit as st

from studio_lib.project import init_project_state, sync_ruleset_from_pack
from studio_lib.theme import STUDIO_CSS
from views import assets, build, mechanics, play, project, scene

st.set_page_config(
    page_title="Hades Studio",
    page_icon="◆",
    layout="wide",
    initial_sidebar_state="expanded",
)

st.markdown(STUDIO_CSS, unsafe_allow_html=True)
init_project_state()

_WORKSPACES = {
    "project": ("Project", "◆"),
    "scene": ("Scene", "▣"),
    "assets": ("Assets", "◇"),
    "mechanics": ("Mechanics", "⚙"),
    "play": ("Play", "▶"),
    "build": ("Build", "⛭"),
}

with st.sidebar:
    st.markdown("## Hades Studio")
    st.caption("TBC Engine · unified editor")

    from forge_lib.packs import list_pack_ids, list_ruleset_ids

    packs = list_pack_ids()
    rulesets = list_ruleset_ids()
    pack_pick = st.selectbox(
        "Active pack",
        packs or ["—"],
        index=packs.index(st.session_state.studio_pack)
        if st.session_state.studio_pack in packs
        else 0,
        key="studio_pack_pick",
    )
    if packs and pack_pick != "—":
        if pack_pick != st.session_state.studio_pack:
            st.session_state.studio_pack = pack_pick
            sync_ruleset_from_pack()
        st.session_state.studio_ruleset = st.selectbox(
            "Project ruleset",
            rulesets or ["pmr.v1"],
            index=rulesets.index(st.session_state.studio_ruleset)
            if st.session_state.studio_ruleset in rulesets
            else 0,
            key="studio_ruleset_pick",
        )
    else:
        st.session_state.studio_pack = None
        st.session_state.studio_ruleset = st.selectbox(
            "Project ruleset",
            rulesets or ["pmr.v1"],
            key="studio_ruleset_only",
        )

    st.divider()
    labels = [f"{_WORKSPACES[k][1]}  {_WORKSPACES[k][0]}" for k in _WORKSPACES]
    keys = list(_WORKSPACES.keys())
    idx = keys.index(st.session_state.studio_workspace)
    pick = st.radio(
        "Editor",
        keys,
        index=idx,
        format_func=lambda k: f"{_WORKSPACES[k][1]}  {_WORKSPACES[k][0]}",
        label_visibility="collapsed",
    )
    st.session_state.studio_workspace = pick

    if st.button("▶ Play", type="primary", use_container_width=True):
        st.session_state.studio_workspace = "play"
        st.rerun()

pack = st.session_state.studio_pack or "no pack"
rs = st.session_state.studio_ruleset
ws_name = _WORKSPACES[st.session_state.studio_workspace][0]
st.markdown(
    f'<div class="studio-toolbar">'
    f'<span class="studio-toolbar-title">{ws_name}</span>'
    f'<span class="studio-toolbar-sub">{pack} · {rs}</span>'
    f"</div>",
    unsafe_allow_html=True,
)

ws = st.session_state.studio_workspace
if ws == "project":
    project.render_project()
elif ws == "scene":
    scene.render_scene()
elif ws == "assets":
    assets.render_assets()
elif ws == "mechanics":
    mechanics.render_mechanics()
elif ws == "play":
    play.render_play()
elif ws == "build":
    build.render_build()
else:
    project.render_project()
