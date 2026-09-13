"""Mechanics Forge — catalog, ruleset designer, and custom mechanic extensions."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import streamlit as st

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from mech_lib.catalog import by_category, categories
from mech_lib.editor import read_widget_defaults, sync_draft_from_widgets
from mech_lib.extensions import (
    list_extensions,
    load_extension,
    new_extension,
    save_extension,
    validate_extension,
)
from mech_lib.paths import EXTENSIONS_DIR, RULESETS_DIR
from mech_lib.ruleset_io import (
    PSI_SCOPES,
    apply_patches,
    blank_from_template,
    list_ruleset_ids,
    load_ruleset_by_id,
    save_ruleset,
    validate_ruleset,
)

st.set_page_config(
    page_title="Mechanics Forge",
    page_icon="⚙",
    layout="wide",
)

st.title("Mechanics Forge")
st.caption("Design rulesets and custom mechanics for the TBC engine — what ships today, plus your additions.")

if "draft" not in st.session_state:
    ids = list_ruleset_ids()
    base = ids[0] if ids else "pmr.v1"
    try:
        st.session_state.draft = load_ruleset_by_id(base)
    except OSError:
        st.session_state.draft = {"id": "pmr.custom.v1", "title": "Custom"}

tab_catalog, tab_designer, tab_custom, tab_export = st.tabs(
    ["Engine catalog", "Ruleset designer", "Custom additions", "Export & validate"]
)

with tab_catalog:
    st.subheader("Implemented in `tbc-engine`")
    cat = st.selectbox("Category", ["all"] + categories(), key="catalog_category")
    for m in by_category(cat if cat != "all" else None):
        status = "✅ engine" if m.engine_implemented else "⚠ partial"
        path = f"`{m.ruleset_path}`" if m.ruleset_path else "_(runtime only)_"
        intent = f" · intent `{m.intent_verb}`" if m.intent_verb else ""
        with st.expander(f"{m.name} — {status}"):
            st.markdown(m.description)
            st.markdown(f"**Ruleset field:** {path}{intent}")
            if m.ruleset_path and st.button("Jump to designer", key=f"jump_{m.id}"):
                st.session_state.designer_focus = m.ruleset_path.split(".")[0]
                st.info("Open the **Ruleset designer** tab to edit this section.")

with tab_designer:
    ids = list_ruleset_ids()
    col_load, col_new = st.columns(2)
    with col_load:
        pick = st.selectbox("Load ruleset", ids or ["pmr.v1"], key="load_pick")
        if st.button("Load into draft"):
            st.session_state.draft = load_ruleset_by_id(pick)
            st.rerun()
    with col_new:
        with st.form("new_ruleset"):
            base = st.selectbox("Template", ids or ["pmr.v1"], key="new_ruleset_template")
            new_id = st.text_input("New id", value="pmr.custom.v1", key="new_ruleset_id")
            new_title = st.text_input("Title", value="My ruleset", key="new_ruleset_title")
            if st.form_submit_button("Create draft from template"):
                st.session_state.draft = blank_from_template(base, new_id, new_title)
                st.rerun()

    w = read_widget_defaults(st.session_state.draft)

    st.markdown("### Core")
    c1, c2, c3, c4 = st.columns(4)
    ruleset_id = c1.text_input("Id", value=w["ruleset_id"], key="designer_ruleset_id")
    title = c2.text_input("Title", value=w["title"], key="designer_title")
    tightness = c3.slider("Tightness", 0.0, 1.0, w["tightness"], key="designer_tightness")
    dt_ms = c4.number_input(
        "dt_ms", min_value=16, max_value=500, value=w["dt_ms"], key="designer_dt_ms"
    )

    st.markdown("### Motion")
    m1, m2, m3, m4 = st.columns(4)
    gravity = m1.number_input(
        "Gravity", value=w["gravity"], format="%.2f", key="designer_gravity"
    )
    max_speed = m2.number_input(
        "Max speed", value=w["max_speed"], format="%.1f", key="designer_max_speed"
    )
    air_control = m3.number_input(
        "Air control", 0.0, 1.0, w["air_control"], key="designer_air_control"
    )
    blink = m4.checkbox("Blink enabled", value=w["blink"], key="designer_blink")
    c_info_val = st.number_input(
        "c_info_m_s (0 = none)",
        min_value=0.0,
        value=float(w["c_info"] or 0.0),
        format="%.0f",
        key="designer_c_info",
    )
    c_info = c_info_val if c_info_val > 0 else None

    st.markdown("### Conservation & death")
    e1, e2, e3 = st.columns(3)
    cons_items = e1.selectbox(
        "Items model",
        ["unique", "clone-tax", "unlimited"],
        index=["unique", "clone-tax", "unlimited"].index(w["cons_items"])
        if w["cons_items"] in ("unique", "clone-tax", "unlimited")
        else 0,
        key="designer_cons_items",
    )
    cons_currency = e2.text_input("Currency", value=w["cons_currency"], key="designer_currency")
    clone_tax = None
    if cons_items == "clone-tax":
        clone_tax = e3.number_input(
            "Clone tax entropy",
            value=float(w["clone_tax"] or 0.002),
            format="%.4f",
            key="designer_clone_tax",
        )
    d1, d2, d3 = st.columns(3)
    death_unbind = d1.checkbox("Unbind on death", value=w["death_unbind"], key="designer_death_unbind")
    death_park = d2.number_input(
        "Park (s)", min_value=0, value=w["death_park"], key="designer_death_park"
    )
    death_rewind = d3.number_input(
        "Rewind (s)", min_value=0, value=w["death_rewind"], key="designer_death_rewind"
    )

    st.markdown("### Psi & sleep")
    p1, p2 = st.columns(2)
    psi_enabled = p1.checkbox("Psi enabled", value=w["psi_enabled"], key="designer_psi_enabled")
    psi_cost = p1.number_input(
        "Psi base cost", value=w["psi_cost"], format="%.4f", key="designer_psi_cost"
    )
    psi_scopes = p2.multiselect(
        "Psi scopes",
        [
            "PastOwn",
            "PastShared",
            "FutureSelf",
            "FutureIsland",
            "RwwQuery",
        ],
        default=[s for s in w["psi_scopes"] if s in PSI_SCOPES],
        key="designer_psi_scopes",
    )
    s1, s2 = st.columns(2)
    sleep_delay = s1.number_input(
        "Sleep delay (s)", value=w["sleep_delay"], key="designer_sleep_delay"
    )
    sleep_kinematic = s2.checkbox(
        "Kinematic wake", value=w["sleep_kinematic"], key="designer_sleep_kinematic"
    )
    handoff_targets = st.text_input(
        "Handoff targets (comma-separated ruleset ids)",
        value=w["handoff_targets"],
        key="designer_handoff_targets",
    )

    st.markdown("### Verbs")
    with st.expander("Attack", expanded=True):
        a1, a2, a3 = st.columns(3)
        atk_enabled = a1.checkbox("Enabled", value=w["atk_enabled"], key="atk_en")
        atk_damage = a2.number_input("Damage", value=w["atk_damage"], key="atk_damage")
        atk_range = a3.number_input("Range (m)", value=w["atk_range"], key="atk_range")
        a4, a5 = st.columns(2)
        atk_stamina = a4.number_input("Stamina cost", value=w["atk_stamina"], key="atk_stamina")
        atk_entropy = a5.number_input("Harm entropy", value=w["atk_entropy"], key="atk_entropy")
    with st.expander("Interact"):
        i1, i2, i3 = st.columns(3)
        int_enabled = i1.checkbox("Enabled", value=w["int_enabled"], key="int_en")
        int_range = i2.number_input("Range (m)", value=w["int_range"], key="int_range")
        int_prefix = i3.text_input("Item prefix", value=w["int_prefix"], key="int_prefix")
    with st.expander("Speak"):
        sp1, sp2 = st.columns(2)
        spk_enabled = sp1.checkbox("Enabled", value=w["spk_enabled"], key="spk_en")
        spk_range = sp2.number_input("Range (m)", value=w["spk_range"], key="spk_range")
    with st.expander("Assist"):
        as1, as2, as3 = st.columns(3)
        ast_enabled = as1.checkbox("Enabled", value=w["ast_enabled"], key="ast_en")
        ast_range = as2.number_input("Range (m)", value=w["ast_range"], key="ast_range")
        ast_heal = as3.number_input("Heal amount", value=w["ast_heal"], key="ast_heal")
        as4, as5, as6 = st.columns(3)
        ast_stamina = as4.number_input("Stamina cost", value=w["ast_stamina"], key="ast_stamina")
        ast_entropy = as5.number_input("Aid entropy", value=w["ast_entropy"], key="ast_entropy")
        ast_ai = as6.checkbox("AI practice", value=w["ast_ai"], key="ast_ai")

    st.markdown("### CRDT (optional)")
    crdt_enabled = st.checkbox("Enable CRDT block", value=w["crdt_enabled"], key="designer_crdt_enabled")
    crdt_props, crdt_presence = w["crdt_props"], w["crdt_presence"]
    if crdt_enabled:
        c1, c2 = st.columns(2)
        crdt_props = c1.text_input("Props model", value=w["crdt_props"], key="designer_crdt_props")
        crdt_presence = c2.text_input(
            "Presence model", value=w["crdt_presence"], key="designer_crdt_presence"
        )

    draft = sync_draft_from_widgets(
        st.session_state.draft,
        ruleset_id=ruleset_id,
        title=title,
        tightness=tightness,
        dt_ms=dt_ms,
        gravity=gravity,
        max_speed=max_speed,
        air_control=air_control,
        blink=blink,
        c_info=c_info,
        cons_items=cons_items,
        cons_currency=cons_currency,
        clone_tax=clone_tax,
        death_unbind=death_unbind,
        death_park=death_park,
        death_rewind=death_rewind,
        psi_enabled=psi_enabled,
        psi_cost=psi_cost,
        psi_scopes=psi_scopes,
        sleep_delay=sleep_delay,
        sleep_kinematic=sleep_kinematic,
        handoff_targets=handoff_targets,
        atk_enabled=atk_enabled,
        atk_damage=atk_damage,
        atk_range=atk_range,
        atk_stamina=atk_stamina,
        atk_entropy=atk_entropy,
        int_enabled=int_enabled,
        int_range=int_range,
        int_prefix=int_prefix,
        spk_enabled=spk_enabled,
        spk_range=spk_range,
        ast_enabled=ast_enabled,
        ast_range=ast_range,
        ast_stamina=ast_stamina,
        ast_heal=ast_heal,
        ast_entropy=ast_entropy,
        ast_ai=ast_ai,
        crdt_enabled=crdt_enabled,
        crdt_props=crdt_props,
        crdt_presence=crdt_presence,
    )
    st.session_state.draft = draft

    st.json(draft)

with tab_custom:
    st.subheader("Custom mechanic extensions")
    st.markdown(
        f"Saved under `{EXTENSIONS_DIR}`. "
        "**ruleset_patch** extensions merge into a ruleset on apply; "
        "**design_only** tracks ideas until the engine gains support."
    )

    ext_paths = list_extensions()
    if ext_paths:
        pick_ext = st.selectbox(
            "Load extension",
            ext_paths,
            format_func=lambda p: p.stem,
            key="ext_pick",
        )
        if st.button("Load extension into editor"):
            st.session_state.ext_draft = load_extension(pick_ext)
            st.rerun()

    if "ext_draft" not in st.session_state:
        st.session_state.ext_draft = new_extension("my_mechanic", "My Mechanic")

    ext = st.session_state.ext_draft
    ext["id"] = st.text_input("Extension id", value=ext.get("id", "custom.my"), key="ext_id")
    ext["title"] = st.text_input("Title", value=ext.get("title", ""), key="ext_title")
    ext["description"] = st.text_area(
        "Description", value=ext.get("description", ""), key="ext_description"
    )
    ext["category"] = st.text_input("Category", value=ext.get("category", "custom"), key="ext_category")
    ext["engine_status"] = st.selectbox(
        "Status",
        ["ruleset_patch", "design_only", "planned"],
        index=["ruleset_patch", "design_only", "planned"].index(
            ext.get("engine_status", "ruleset_patch")
        ),
        key="ext_engine_status",
    )
    compat_str = st.text_input(
        "Compatible rulesets (comma)",
        value=", ".join(ext.get("compatible_rulesets", [])),
        key="ext_compatible",
    )
    ext["compatible_rulesets"] = [x.strip() for x in compat_str.split(",") if x.strip()]

    st.markdown("**Patches** (dotted paths → values)")
    patches: dict = ext.setdefault("patches", {})
    rows = st.number_input(
        "Patch rows",
        min_value=1,
        max_value=20,
        value=max(3, len(patches) or 3),
        key="ext_patch_rows",
    )
    new_patches = {}
    existing_items = list(patches.items())
    for i in range(int(rows)):
        default_path, default_val = ("", "")
        if i < len(existing_items):
            default_path, default_val = existing_items[i]
            default_val = json.dumps(default_val) if not isinstance(default_val, str) else default_val
        c1, c2 = st.columns([1, 1])
        path = c1.text_input(f"Path {i+1}", value=default_path, key=f"path_{i}")
        raw = c2.text_input(f"Value {i+1} (JSON)", value=str(default_val), key=f"val_{i}")
        if path.strip():
            try:
                new_patches[path.strip()] = json.loads(raw)
            except json.JSONDecodeError:
                new_patches[path.strip()] = raw
    ext["patches"] = new_patches
    ext["design_notes"] = st.text_area(
        "Design notes (one per line)",
        value="\n".join(ext.get("design_notes", [])),
        key="ext_design_notes",
    ).splitlines()

    st.session_state.ext_draft = ext

    b1, b2, b3 = st.columns(3)
    if b1.button("Save extension"):
        errs = validate_extension(ext)
        if errs:
            st.error("\n".join(errs))
        else:
            path = save_extension(ext)
            st.success(f"Saved {path}")
    if b2.button("Apply patches to ruleset draft"):
        st.session_state.draft = apply_patches(st.session_state.draft, ext["patches"])
        st.success("Merged into ruleset draft — review in designer / export tab.")
    if b3.button("New blank extension"):
        st.session_state.ext_draft = new_extension("mechanic", "New Mechanic")
        st.rerun()

with tab_export:
    draft = st.session_state.draft
    errs = validate_ruleset(draft)
    if errs:
        st.warning("Validation:\n" + "\n".join(f"- {e}" for e in errs))
    else:
        st.success("Draft passes structural validation.")

    st.code(json.dumps(draft, indent=2), language="json")

    if st.button("Save ruleset to repo", type="primary"):
        if errs:
            st.error("Fix validation errors before saving.")
        else:
            path = save_ruleset(draft)
            st.success(f"Wrote `{path}` — levels using `{draft['id']}` will pick this up on next run.")

    st.markdown(
        f"""
        **Where this lands**

        - Rulesets: `{RULESETS_DIR}`
        - Content packs reference `ruleset_id` in `assets/packs/*/manifest.json`
        - Play with **tbc-play** (`run-play.bat`) after saving
        """
    )
