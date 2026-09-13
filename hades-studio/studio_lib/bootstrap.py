"""Path setup so Studio can import asset-forge, mechanics-forge, and dashboard."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
STUDIO_ROOT = REPO_ROOT / "hades-studio"
ASSET_FORGE = REPO_ROOT / "asset-forge"
MECHANICS_FORGE = REPO_ROOT / "mechanics-forge"
DASHBOARD = REPO_ROOT / "dashboard"


def install_paths() -> Path:
    for p in (STUDIO_ROOT, ASSET_FORGE, MECHANICS_FORGE, DASHBOARD):
        s = str(p)
        if s not in sys.path:
            sys.path.insert(0, s)
    return REPO_ROOT
