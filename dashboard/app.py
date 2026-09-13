"""Local CI dashboard for hades-engine."""

from __future__ import annotations

import streamlit as st

from runner import (
    CI_JOBS,
    EXTRA_JOBS,
    REPO_ROOT,
    git_info,
    gh_ci_runs,
    job_by_id,
    load_history,
    run_ci_pipeline,
    run_job,
)

st.set_page_config(
    page_title="Hades Engine — Dev Dashboard",
    page_icon="⚙",
    layout="wide",
    initial_sidebar_state="expanded",
)

st.markdown(
    """
    <style>
    .block-container { padding-top: 1.5rem; }
    div[data-testid="stMetricValue"] { font-size: 1.1rem; }
    </style>
    """,
    unsafe_allow_html=True,
)

branch, sha, dirty = git_info()
history = load_history()
recent = list(reversed(history[-30:]))

st.title("Hades Engine — Dev Dashboard")
st.caption(f"Repo: `{REPO_ROOT}`")

col1, col2, col3, col4 = st.columns(4)
col1.metric("Branch", branch)
col2.metric("Commit", sha)
col3.metric("Working tree", "dirty" if dirty else "clean")
last_ok = next((h for h in reversed(history) if h.get("success")), None)
col4.metric(
    "Last green run",
    last_ok.get("job_label", "—") if last_ok else "—",
)

st.divider()

side = st.sidebar
side.header("Run jobs")
side.markdown("Matches [`.github/workflows/ci.yml`](.github/workflows/ci.yml) order.")

if side.button("Run full CI pipeline", type="primary", use_container_width=True):
    st.session_state["pending_run"] = ("ci", None)

side.markdown("**CI steps**")
for job in CI_JOBS:
    if side.button(job.label, key=f"btn_{job.id}", use_container_width=True):
        st.session_state["pending_run"] = ("single", job.id)

side.markdown("**Extra**")
for job in EXTRA_JOBS:
    if side.button(job.label, key=f"btn_x_{job.id}", use_container_width=True):
        st.session_state["pending_run"] = ("single", job.id)

if side.button("Clear session log", use_container_width=True):
    st.session_state.pop("log_lines", None)
    st.session_state.pop("last_results", None)

tab_overview, tab_history, tab_remote, tab_help = st.tabs(
    ["Overview", "Run history", "GitHub CI", "Help"]
)

with tab_overview:
    st.subheader("CI steps")
    for job in CI_JOBS:
        st.markdown(f"- **{job.label}** — `{ ' '.join(job.command) }`")
        st.caption(job.description)

    st.subheader("Quick reference")
    st.code(
        """cargo test -p tbc-engine
cargo build -p tbc-engine --release
cargo fmt --all -- --check""",
        language="bash",
    )

with tab_history:
    if not recent:
        st.info("No local runs yet. Use the sidebar to run CI or a single job.")
    else:
        st.dataframe(
            [
                {
                    "When": h.get("started_at", "")[:19].replace("T", " "),
                    "Job": h.get("job_label"),
                    "OK": h.get("success"),
                    "Sec": h.get("duration_sec"),
                    "Branch": h.get("git_branch"),
                    "SHA": h.get("git_sha"),
                }
                for h in recent
            ],
            use_container_width=True,
            hide_index=True,
        )
        with st.expander("View log for latest run"):
            if recent:
                st.code(recent[0].get("log_tail", ""), language="text")

with tab_remote:
    runs = gh_ci_runs()
    if runs is None:
        st.warning(
            "Could not load GitHub Actions runs. Install [GitHub CLI](https://cli.github.com/) "
            "and run `gh auth login`, or check network access."
        )
    elif not runs:
        st.info("No workflow runs returned for ci.yml.")
    else:
        for r in runs:
            conclusion = r.get("conclusion") or r.get("status") or "?"
            icon = "✅" if conclusion == "success" else "❌" if conclusion == "failure" else "⏳"
            st.markdown(
                f"{icon} **{conclusion}** — `{r.get('headBranch')}` — "
                f"[run]({r.get('url')}) — {r.get('createdAt', '')[:19]}"
            )

with tab_help:
    st.markdown(
        """
        ### Start the dashboard

        **Windows:** double-click `run-dashboard.bat` in the repo root, or:

        ```powershell
        .\\run-dashboard.ps1
        ```

        First launch creates `dashboard\\.venv` and installs Streamlit.

        ### What gets tracked

        Each job appends to `dashboard/.data/history.json` (gitignored): time, pass/fail,
        duration, git branch/SHA, and the tail of the log output.

        ### Requirements

        - [Rust toolchain](https://rustup.rs/) (`cargo` on PATH)
        - Python 3.10+
        - Optional: `gh` for the GitHub CI tab
        """
    )

pending = st.session_state.pop("pending_run", None)

if pending:
    run_mode, run_job_id = pending
    st.divider()
    st.subheader("Live run")
    log_box = st.empty()
    status_box = st.empty()
    lines: list[str] = st.session_state.get("log_lines", [])

    def append_line(line: str) -> None:
        lines.append(line.rstrip("\n"))
        if len(lines) > 4000:
            del lines[: len(lines) - 4000]
        st.session_state["log_lines"] = lines
        log_box.code("\n".join(lines[-80:]), language="text")

    if run_mode == "ci":
        status_box.info("Running CI pipeline…")

        def on_job_start(job) -> None:
            append_line(f"\n=== {job.label} ===\n")

        results = run_ci_pipeline(on_line=append_line, on_job_start=on_job_start)
        st.session_state["last_results"] = results
        failed = [r for r in results if not r.success]
        if failed:
            status_box.error(f"CI stopped at: {failed[0].job_label}")
        elif results:
            status_box.success("CI pipeline finished successfully.")
        else:
            status_box.warning("No jobs ran.")
        st.code("\n".join(lines[-200:]), language="text")

    elif run_mode == "single" and run_job_id:
        job = job_by_id(run_job_id)
        if not job:
            status_box.error("Unknown job.")
        else:
            status_box.info(f"Running: {job.label}")
            record = run_job(job, on_line=append_line)
            st.session_state["last_results"] = [record]
            if record.success:
                status_box.success(f"{job.label} passed ({record.duration_sec}s)")
            else:
                status_box.error(
                    f"{job.label} failed (exit {record.exit_code}, {record.duration_sec}s)"
                )
            st.code(record.log_tail or "\n".join(lines[-200:]), language="text")
