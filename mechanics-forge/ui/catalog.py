from __future__ import annotations

import streamlit as st

from mech_lib.catalog import ENGINE_CATALOG, by_category, categories


def render_catalog() -> None:
    st.markdown("### Engine mechanics catalog")
    q = st.text_input("Search", placeholder="attack, psi, motion…", key="cat_search")
    cat = st.selectbox("Category", ["all"] + categories(), key="cat_filter")

    rows = []
    for m in by_category(cat if cat != "all" else None):
        hay = f"{m.name} {m.description} {m.ruleset_path or ''} {m.intent_verb or ''}".lower()
        if q and q.lower() not in hay:
            continue
        rows.append(
            {
                "Name": m.name,
                "Category": m.category,
                "Ruleset path": m.ruleset_path or "—",
                "Intent": m.intent_verb or "—",
                "Engine": "yes" if m.engine_implemented else "partial",
            }
        )

    st.dataframe(rows, use_container_width=True, hide_index=True, height=420)
    st.caption(f"{len(rows)} / {len(ENGINE_CATALOG)} entries")

    pick = st.selectbox(
        "Detail",
        [m.id for m in ENGINE_CATALOG],
        format_func=lambda i: next(m.name for m in ENGINE_CATALOG if m.id == i),
        key="cat_detail",
    )
    m = next(x for x in ENGINE_CATALOG if x.id == pick)
    st.markdown(f"**{m.name}** — {m.description}")
    if m.ruleset_path:
        st.code(m.ruleset_path, language="text")
