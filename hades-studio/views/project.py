from __future__ import annotations

import streamlit as st

from forge_lib.packs import list_pack_ids, list_ruleset_ids, load_manifest
from forge_lib.validate import validate_all
from studio_lib.project import project_summary


def render_project() -> None:
    st.markdown("### Project hub")
    summary = project_summary()
    c1, c2, c3 = st.columns(3)
    c1.metric("Content pack", summary["pack"] or "—")
    c2.metric("Ruleset", summary["ruleset"])
    packs = list_pack_ids()
    c3.metric("Packs in repo", len(packs))

    st.markdown("#### Pipeline")
    st.markdown(
        """
        1. **Scene** — place entities, edit transforms on the map  
        2. **Assets** — procedural sprites, export into the active pack  
        3. **Mechanics** — rulesets, combat FX extensions, verbs  
        4. **Play** — launch `tbc-play` with your pack + ruleset  
        5. **Build** — tests, clippy, release build (same as CI)
        """
    )

    if summary["pack"]:
        try:
            m = load_manifest(summary["pack"])
            st.json({"manifest": m})
        except OSError as exc:
            st.error(str(exc))

    errs = validate_all()
    if errs:
        st.warning("Validation notes:\n" + "\n".join(errs[:12]))
    else:
        st.success("All packs validate.")

    st.caption(f"Rulesets on disk: {', '.join(list_ruleset_ids()) or 'none'}")
