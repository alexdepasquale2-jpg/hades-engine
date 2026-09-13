from __future__ import annotations

import streamlit as st

from runner import (
    CI_JOBS,
    cargo_available,
    git_info,
    is_streamlit_cloud,
    load_history,
    run_ci_pipeline,
    run_job,
    toolchain_hint,
)


def render_build() -> None:
    st.markdown("### Build & CI")
    can_run = cargo_available()
    if not can_run:
        st.warning(toolchain_hint())
    elif is_streamlit_cloud():
        st.info("Run builds locally; hosted Studio cannot invoke `cargo`.")

    branch, sha, dirty = git_info()
    c1, c2, c3 = st.columns(3)
    c1.metric("Branch", branch)
    c2.metric("Commit", sha)
    c3.metric("Tree", "dirty" if dirty else "clean")

    history = load_history()
    if st.button("Run full CI pipeline", type="primary", disabled=not can_run):
        with st.spinner("Running CI jobs…"):
            results = run_ci_pipeline()
        st.session_state.last_ci = [
            {"job": r.job_label, "ok": r.success, "sec": r.duration_sec} for r in results
        ]
        st.rerun()

    if st.session_state.get("last_ci"):
        st.dataframe(st.session_state.last_ci, hide_index=True)

    st.markdown("#### Individual jobs")
    for job in CI_JOBS:
        if st.button(f"Run {job.label}", key=f"job_{job.id}", disabled=not can_run):
            with st.spinner(job.label):
                rec = run_job(job)
            st.code(rec.log_tail[:8000] or "(no output)")

    with st.expander("Recent history"):
        for entry in reversed(history[-15:]):
            ok = "✓" if entry.get("success") else "✗"
            st.caption(f"{ok} {entry.get('job_id', '?')} — {entry.get('finished_at', '')}")
