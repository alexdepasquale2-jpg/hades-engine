from __future__ import annotations

import os
import subprocess
import sys

import streamlit as st

from studio_lib.bootstrap import REPO_ROOT


def render_play() -> None:
    pack = st.session_state.studio_pack
    ruleset = st.session_state.studio_ruleset

    st.markdown("### Play mode")
    st.markdown(
        f"""
        **Active pack:** `{pack or '—'}`  
        **Ruleset:** `{ruleset}`  

        The native client loads all packs under `assets/packs/` and rulesets under `rulesets/`.
        Ensure your pack manifest `ruleset_id` matches the ruleset you edited in **Mechanics**.
        """
    )

    col1, col2 = st.columns(2)
    with col1:
        if st.button("Launch tbc-play (Windows)", type="primary", use_container_width=True):
            bat = REPO_ROOT / "run-play.bat"
            if not bat.is_file():
                st.error(f"Missing `{bat}`")
            else:
                env = os.environ.copy()
                env["HADES_REPO_ROOT"] = str(REPO_ROOT)
                subprocess.Popen(
                    ["cmd", "/c", "start", "", str(bat)],
                    cwd=str(REPO_ROOT),
                    env=env,
                )
                st.success("Started game window (see taskbar).")
    with col2:
        st.code(
            f"cd {REPO_ROOT}\ncargo run -p tbc-game --release",
            language="bash",
        )

    st.markdown("#### Controls")
    st.markdown(
        "**↑↓** level select · **Enter** play · **WASD** move · **E** interact · "
        "**Space** attack · **Esc** menu"
    )

    if pack:
        st.info(
            f"In-game, pick the level that uses `{pack}`. "
            "Combat tuning comes from the ruleset JSON; visual FX need client support for `presentation.combat`."
        )
