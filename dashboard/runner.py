"""Run repo jobs (cargo / git) and persist history."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

REPO_ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = Path(__file__).resolve().parent / ".data"
HISTORY_FILE = DATA_DIR / "history.json"
MAX_HISTORY = 200


class ToolchainError(RuntimeError):
    """Required CLI tool (cargo, git, etc.) is not available."""


def _extra_bin_dirs() -> list[Path]:
    dirs: list[Path] = []
    home = Path.home()
    dirs.append(home / ".cargo" / "bin")
    if sys.platform == "win32":
        local = os.environ.get("LOCALAPPDATA")
        if local:
            dirs.append(Path(local) / "cargo" / "bin")
    return dirs


def find_executable(name: str) -> str | None:
    found = shutil.which(name)
    if found:
        return found
    suffix = ".exe" if sys.platform == "win32" else ""
    for directory in _extra_bin_dirs():
        candidate = directory / f"{name}{suffix}"
        if candidate.is_file():
            return str(candidate)
    return None


def is_streamlit_cloud() -> bool:
    if os.environ.get("STREAMLIT_RUNTIME_ENV") == "cloud":
        return True
    root = REPO_ROOT.as_posix()
    return root.startswith("/mount/src/")


def cargo_available() -> bool:
    return find_executable("cargo") is not None


def toolchain_hint() -> str:
    if is_streamlit_cloud():
        return (
            "This hosted Streamlit app cannot run `cargo` (no Rust toolchain in the cloud "
            "runtime). Use the **GitHub CI** tab to track builds, or run the dashboard "
            "on your PC with `run-dashboard.bat` to execute tests and builds locally."
        )
    return (
        "Install the [Rust toolchain](https://rustup.rs/) and ensure `cargo` is on your PATH, "
        "then restart the dashboard. On Windows, open a new terminal after installing rustup."
    )


def resolve_command(command: list[str]) -> list[str]:
    if not command:
        raise ToolchainError("Empty command.")
    exe = find_executable(command[0])
    if exe is None:
        raise ToolchainError(
            f"Cannot find `{command[0]}` on PATH. {toolchain_hint()}"
        )
    return [exe, *command[1:]]


@dataclass
class JobDef:
    id: str
    label: str
    command: list[str]
    description: str


CI_JOBS: list[JobDef] = [
    JobDef(
        id="fmt",
        label="Format check",
        command=["cargo", "fmt", "--all", "--", "--check"],
        description="Same as CI: cargo fmt --all -- --check",
    ),
    JobDef(
        id="build_release",
        label="Build (release)",
        command=["cargo", "build", "-p", "tbc-engine", "--release"],
        description="Release build of tbc-engine",
    ),
    JobDef(
        id="test",
        label="Tests",
        command=["cargo", "test", "-p", "tbc-engine"],
        description="Full tbc-engine test suite",
    ),
    JobDef(
        id="publish_dry",
        label="Packaging (dry-run)",
        command=["cargo", "publish", "-p", "tbc-engine", "--dry-run"],
        description="Verify crate publishes without uploading",
    ),
    JobDef(
        id="clippy",
        label="Clippy",
        command=["cargo", "clippy", "-p", "tbc-engine", "--all-targets"],
        description="Lint all targets (CI allows warnings)",
    ),
]

EXTRA_JOBS: list[JobDef] = [
    JobDef(
        id="build_debug",
        label="Build (debug)",
        command=["cargo", "build", "-p", "tbc-engine"],
        description="Fast debug build",
    ),
    JobDef(
        id="bench",
        label="Bench tick_step",
        command=["cargo", "bench", "-p", "tbc-engine", "--bench", "tick_step"],
        description="Run tick_step benchmark (may take a while)",
    ),
]


@dataclass
class RunRecord:
    run_id: str
    job_id: str
    job_label: str
    started_at: str
    finished_at: str
    duration_sec: float
    exit_code: int
    success: bool
    git_branch: str
    git_sha: str
    log_tail: str


def _ensure_data_dir() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)


def load_history() -> list[dict]:
    _ensure_data_dir()
    if not HISTORY_FILE.exists():
        return []
    try:
        return json.loads(HISTORY_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return []


def save_history(entries: list[dict]) -> None:
    _ensure_data_dir()
    HISTORY_FILE.write_text(
        json.dumps(entries[-MAX_HISTORY:], indent=2), encoding="utf-8"
    )


def append_record(record: RunRecord) -> None:
    entries = load_history()
    entries.append(asdict(record))
    save_history(entries)


def git_info() -> tuple[str, str, bool]:
    git = find_executable("git")
    if not git:
        return "unknown", "unknown", False

    def run(args: list[str]) -> str:
        try:
            out = subprocess.run(
                [git, *args],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                timeout=30,
                shell=False,
            )
            return (out.stdout or "").strip()
        except (subprocess.TimeoutExpired, OSError):
            return ""

    branch = run(["rev-parse", "--abbrev-ref", "HEAD"]) or "unknown"
    sha = run(["rev-parse", "--short", "HEAD"]) or "unknown"
    dirty = bool(run(["status", "--porcelain"]))
    return branch, sha, dirty


def stream_command(
    command: list[str],
    on_line: Callable[[str], None] | None = None,
    env_extra: dict[str, str] | None = None,
) -> tuple[int, str]:
    resolved = resolve_command(command)
    env = os.environ.copy()
    env.setdefault("CARGO_TERM_COLOR", "always")
    env.setdefault("RUST_BACKTRACE", "1")
    cargo_home = find_executable("cargo")
    if cargo_home:
        cargo_bin = str(Path(cargo_home).parent)
        env["PATH"] = cargo_bin + os.pathsep + env.get("PATH", "")
    if env_extra:
        env.update(env_extra)

    proc = subprocess.Popen(
        resolved,
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
        env=env,
        shell=False,
    )
    lines: list[str] = []
    assert proc.stdout is not None
    for line in proc.stdout:
        lines.append(line)
        if on_line:
            on_line(line)
    proc.wait()
    full = "".join(lines)
    return proc.returncode or 0, full


def run_job(
    job: JobDef,
    on_line: Callable[[str], None] | None = None,
) -> RunRecord:
    branch, sha, _ = git_info()
    started = datetime.now(timezone.utc)
    t0 = time.perf_counter()
    code, log = stream_command(job.command, on_line=on_line)
    duration = time.perf_counter() - t0
    finished = datetime.now(timezone.utc)
    tail = log[-12000:] if len(log) > 12000 else log
    record = RunRecord(
        run_id=str(uuid.uuid4()),
        job_id=job.id,
        job_label=job.label,
        started_at=started.isoformat(),
        finished_at=finished.isoformat(),
        duration_sec=round(duration, 2),
        exit_code=code,
        success=code == 0,
        git_branch=branch,
        git_sha=sha,
        log_tail=tail,
    )
    append_record(record)
    return record


def run_ci_pipeline(
    on_line: Callable[[str], None] | None = None,
    on_job_start: Callable[[JobDef], None] | None = None,
) -> list[RunRecord]:
    results: list[RunRecord] = []
    for job in CI_JOBS:
        if on_job_start:
            on_job_start(job)

        def line_cb(line: str, prefix: str = job.label) -> None:
            if on_line:
                on_line(f"[{prefix}] {line}")

        record = run_job(job, on_line=line_cb)
        results.append(record)
        if not record.success:
            break
    return results


def job_by_id(job_id: str) -> JobDef | None:
    for j in CI_JOBS + EXTRA_JOBS:
        if j.id == job_id:
            return j
    return None


def gh_ci_runs(limit: int = 8) -> list[dict] | None:
    gh = find_executable("gh")
    if not gh:
        return None
    try:
        out = subprocess.run(
            [
                gh,
                "run",
                "list",
                "--workflow",
                "ci.yml",
                "--limit",
                str(limit),
                "--json",
                "databaseId,status,conclusion,headBranch,createdAt,url",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=45,
            shell=False,
        )
        if out.returncode != 0:
            return None
        return json.loads(out.stdout or "[]")
    except (subprocess.TimeoutExpired, OSError, json.JSONDecodeError):
        return None


def _github_repo_slug() -> str:
    git = find_executable("git")
    if git:
        try:
            out = subprocess.run(
                [git, "remote", "get-url", "origin"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                timeout=15,
                shell=False,
            )
            url = (out.stdout or "").strip()
            if url.endswith(".git"):
                url = url[:-4]
            if "github.com" in url:
                path = url.split("github.com", 1)[-1].strip(":/")
                if path.count("/") >= 1:
                    return path
        except (subprocess.TimeoutExpired, OSError):
            pass
    return "alexdepasquale2-jpg/hades-engine"


def github_api_ci_runs(limit: int = 8) -> list[dict] | None:
    repo = _github_repo_slug()
    url = (
        f"https://api.github.com/repos/{repo}/actions/workflows/ci.yml/runs"
        f"?per_page={limit}"
    )
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "hades-engine-dashboard",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
    except (urllib.error.URLError, json.JSONDecodeError, OSError):
        return None
    runs: list[dict] = []
    for item in data.get("workflow_runs", []):
        runs.append(
            {
                "conclusion": item.get("conclusion"),
                "status": item.get("status"),
                "headBranch": item.get("head_branch"),
                "createdAt": item.get("created_at"),
                "url": item.get("html_url"),
            }
        )
    return runs


def ci_runs(limit: int = 8) -> tuple[list[dict], str]:
    """Return workflow runs and source label: gh, api, or none."""
    via_gh = gh_ci_runs(limit)
    if via_gh is not None:
        return via_gh, "gh"
    via_api = github_api_ci_runs(limit)
    if via_api is not None:
        return via_api, "api"
    return [], "none"
