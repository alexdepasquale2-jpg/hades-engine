from __future__ import annotations

import streamlit as st

from mech_lib import history
from mech_lib.diff import diff_rulesets
from mech_lib.editor import read_widget_defaults, summary_metrics, sync_draft_from_widgets
from mech_lib.ruleset_io import PSI_SCOPES, validate_ruleset

SECTIONS = ["Core", "Motion", "World", "Psi & sleep", "Verbs", "CRDT"]


def render_workbench() -> None:
    if "ui_section" not in st.session_state:
        st.session_state.ui_section = "Core"

    draft = st.session_state.draft
    w = read_widget_defaults(draft)
    base = st.session_state.baseline_draft or draft

    left, right = st.columns([1.35, 1], gap="large")

    with left:
        st.session_state.ui_section = st.radio(
            "Section",
            SECTIONS,
            horizontal=True,
            key="wb_section_radio",
            label_visibility="collapsed",
        )
        section = st.session_state.ui_section

        if section == "Core":
            with st.form("form_core", border=True):
                st.markdown("#### Core simulation")
                c1, c2 = st.columns(2)
                ruleset_id = c1.text_input("Id", value=w["ruleset_id"])
                title = c2.text_input("Title", value=w["title"])
                tightness = st.slider("Tightness", 0.0, 1.0, w["tightness"])
                dt_ms = st.number_input("dt_ms", min_value=16, max_value=500, value=w["dt_ms"])
                if st.form_submit_button("Apply core", type="primary"):
                    _apply_partial(
                        w,
                        ruleset_id=ruleset_id,
                        title=title,
                        tightness=tightness,
                        dt_ms=dt_ms,
                    )

        elif section == "Motion":
            with st.form("form_motion", border=True):
                st.markdown("#### Motion envelope")
                m1, m2 = st.columns(2)
                gravity = m1.number_input("Gravity", value=w["gravity"], format="%.2f")
                max_speed = m2.number_input("Max speed", value=w["max_speed"], format="%.1f")
                air_control = st.slider("Air control", 0.0, 1.0, w["air_control"])
                blink = st.checkbox("Blink teleport", value=w["blink"])
                c_info_val = st.number_input(
                    "c_info_m_s (0 = off)",
                    min_value=0.0,
                    value=float(w["c_info"] or 0.0),
                )
                if st.form_submit_button("Apply motion", type="primary"):
                    _apply_partial(
                        w,
                        gravity=gravity,
                        max_speed=max_speed,
                        air_control=air_control,
                        blink=blink,
                        c_info=c_info_val if c_info_val > 0 else None,
                    )

        elif section == "World":
            with st.form("form_world", border=True):
                st.markdown("#### Economy, death, handoff")
                cons_items = st.selectbox(
                    "Items model",
                    ["unique", "clone-tax", "unlimited"],
                    index=["unique", "clone-tax", "unlimited"].index(w["cons_items"])
                    if w["cons_items"] in ("unique", "clone-tax", "unlimited")
                    else 0,
                )
                cons_currency = st.text_input("Currency", value=w["cons_currency"])
                clone_tax = None
                if cons_items == "clone-tax":
                    clone_tax = st.number_input(
                        "Clone tax entropy",
                        value=float(w["clone_tax"] or 0.002),
                        format="%.4f",
                    )
                death_unbind = st.checkbox("Unbind on death", value=w["death_unbind"])
                d1, d2 = st.columns(2)
                death_park = d1.number_input("Park (s)", min_value=0, value=w["death_park"])
                death_rewind = d2.number_input("Rewind (s)", min_value=0, value=w["death_rewind"])
                handoff_targets = st.text_input("Handoff targets (csv)", value=w["handoff_targets"])
                if st.form_submit_button("Apply world", type="primary"):
                    _apply_partial(
                        w,
                        cons_items=cons_items,
                        cons_currency=cons_currency,
                        clone_tax=clone_tax,
                        death_unbind=death_unbind,
                        death_park=death_park,
                        death_rewind=death_rewind,
                        handoff_targets=handoff_targets,
                    )

        elif section == "Psi & sleep":
            with st.form("form_psi", border=True):
                st.markdown("#### Psi & LOD sleep")
                psi_enabled = st.checkbox("Psi enabled", value=w["psi_enabled"])
                psi_cost = st.number_input("Psi base cost", value=w["psi_cost"], format="%.4f")
                psi_scopes = st.multiselect(
                    "Scopes",
                    PSI_SCOPES,
                    default=[s for s in w["psi_scopes"] if s in PSI_SCOPES],
                )
                s1, s2 = st.columns(2)
                sleep_delay = s1.number_input("Sleep delay (s)", value=w["sleep_delay"])
                sleep_kinematic = s2.checkbox("Kinematic wake", value=w["sleep_kinematic"])
                if st.form_submit_button("Apply psi/sleep", type="primary"):
                    _apply_partial(
                        w,
                        psi_enabled=psi_enabled,
                        psi_cost=psi_cost,
                        psi_scopes=psi_scopes,
                        sleep_delay=sleep_delay,
                        sleep_kinematic=sleep_kinematic,
                    )

        elif section == "Verbs":
            with st.form("form_verbs", border=True):
                st.markdown("#### Verb policies")
                st.caption("Combat · economy · social channels")
                with st.expander("Attack", expanded=True):
                    atk_enabled = st.checkbox("Enabled", value=w["atk_enabled"], key="wb_atk_en")
                    a1, a2, a3 = st.columns(3)
                    atk_damage = a1.number_input("Damage", value=w["atk_damage"])
                    atk_range = a2.number_input("Range (m)", value=w["atk_range"])
                    atk_stamina = a3.number_input("Stamina", value=w["atk_stamina"])
                    atk_entropy = st.number_input("Harm entropy", value=w["atk_entropy"])
                with st.expander("Interact"):
                    int_enabled = st.checkbox("Enabled", value=w["int_enabled"], key="wb_int_en")
                    i1, i2 = st.columns(2)
                    int_range = i1.number_input("Range (m)", value=w["int_range"], key="wb_int_rng")
                    int_prefix = i2.text_input("Item prefix", value=w["int_prefix"])
                with st.expander("Speak"):
                    spk_enabled = st.checkbox("Enabled", value=w["spk_enabled"], key="wb_spk_en")
                    spk_range = st.number_input("Range (m)", value=w["spk_range"], key="wb_spk_rng")
                with st.expander("Assist"):
                    ast_enabled = st.checkbox("Enabled", value=w["ast_enabled"], key="wb_ast_en")
                    ast_range = st.number_input("Range (m)", value=w["ast_range"], key="wb_ast_rng")
                    ast_heal = st.number_input("Heal", value=w["ast_heal"], key="wb_ast_heal")
                    ast_stamina = st.number_input("Stamina cost", value=w["ast_stamina"], key="wb_ast_sta")
                    ast_entropy = st.number_input("Aid entropy", value=w["ast_entropy"], key="wb_ast_ent")
                    ast_ai = st.checkbox("AI practice", value=w["ast_ai"], key="wb_ast_ai")
                if st.form_submit_button("Apply verbs", type="primary"):
                    _apply_partial(
                        w,
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
                    )

        elif section == "CRDT":
            with st.form("form_crdt", border=True):
                st.markdown("#### CRDT (optional)")
                crdt_enabled = st.checkbox("Enable CRDT block", value=w["crdt_enabled"])
                c1, c2 = st.columns(2)
                crdt_props = c1.text_input("Props model", value=w["crdt_props"])
                crdt_presence = c2.text_input("Presence model", value=w["crdt_presence"])
                if st.form_submit_button("Apply CRDT", type="primary"):
                    _apply_partial(
                        w,
                        crdt_enabled=crdt_enabled,
                        crdt_props=crdt_props,
                        crdt_presence=crdt_presence,
                    )

    with right:
        st.markdown("#### Inspector")
        for k, v in summary_metrics(st.session_state.draft).items():
            st.metric(k, v)
        errs = validate_ruleset(st.session_state.draft)
        if errs:
            st.markdown('<span class="forge-chip forge-chip-err">Invalid</span>', unsafe_allow_html=True)
            for e in errs[:6]:
                st.caption(f"• {e}")
        else:
            st.markdown('<span class="forge-chip forge-chip-clean">Valid</span>', unsafe_allow_html=True)

        st.markdown("#### Diff vs baseline")
        rows = diff_rulesets(base, st.session_state.draft)
        if not rows:
            st.caption("No changes from baseline.")
        else:
            st.dataframe(rows[:40], use_container_width=True, hide_index=True, height=280)


def _apply_partial(w: dict, **overrides) -> None:
    merged = {**w, **overrides}
    draft = sync_draft_from_widgets(st.session_state.draft, **merged)
    st.session_state.draft = draft
    history.push(draft)
    st.toast("Section applied")
    st.rerun()
