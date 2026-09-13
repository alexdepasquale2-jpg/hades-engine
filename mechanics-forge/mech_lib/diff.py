from __future__ import annotations

import json
from typing import Any


def _flatten(obj: Any, prefix: str = "") -> dict[str, str]:
    out: dict[str, str] = {}
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{prefix}.{k}" if prefix else k
            out.update(_flatten(v, p))
    elif isinstance(obj, list):
        out[prefix] = json.dumps(obj, sort_keys=True)
    else:
        out[prefix] = json.dumps(obj)
    return out


def diff_rulesets(a: dict[str, Any], b: dict[str, Any]) -> list[dict[str, str]]:
    fa, fb = _flatten(a), _flatten(b)
    keys = sorted(set(fa) | set(fb))
    rows: list[dict[str, str]] = []
    for key in keys:
        va, vb = fa.get(key), fb.get(key)
        if va != vb:
            rows.append(
                {
                    "path": key,
                    "left": va if va is not None else "—",
                    "right": vb if vb is not None else "—",
                }
            )
    return rows


def count_diff(a: dict[str, Any], b: dict[str, Any]) -> int:
    return len(diff_rulesets(a, b))
