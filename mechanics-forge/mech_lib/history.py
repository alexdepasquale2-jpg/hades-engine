from __future__ import annotations

from copy import deepcopy
from typing import Any

import streamlit as st

MAX_HISTORY = 40


def init() -> None:
    if "hist_stack" not in st.session_state:
        st.session_state.hist_stack: list[dict[str, Any]] = []
        st.session_state.hist_index = -1
        st.session_state.baseline_draft: dict[str, Any] | None = None


def set_baseline(draft: dict[str, Any]) -> None:
    st.session_state.baseline_draft = deepcopy(draft)
    st.session_state.hist_stack = [deepcopy(draft)]
    st.session_state.hist_index = 0


def push(draft: dict[str, Any]) -> None:
    init()
    snap = deepcopy(draft)
    stack = st.session_state.hist_stack
    idx = st.session_state.hist_index
    if idx >= 0 and stack and stack[idx] == snap:
        return
    stack = stack[: idx + 1]
    stack.append(snap)
    if len(stack) > MAX_HISTORY:
        stack = stack[-MAX_HISTORY:]
    st.session_state.hist_stack = stack
    st.session_state.hist_index = len(stack) - 1


def can_undo() -> bool:
    init()
    return st.session_state.hist_index > 0


def can_redo() -> bool:
    init()
    return st.session_state.hist_index < len(st.session_state.hist_stack) - 1


def undo() -> dict[str, Any] | None:
    init()
    if not can_undo():
        return None
    st.session_state.hist_index -= 1
    return deepcopy(st.session_state.hist_stack[st.session_state.hist_index])


def redo() -> dict[str, Any] | None:
    init()
    if not can_redo():
        return None
    st.session_state.hist_index += 1
    return deepcopy(st.session_state.hist_stack[st.session_state.hist_index])


def is_dirty(draft: dict[str, Any]) -> bool:
    init()
    base = st.session_state.baseline_draft
    if base is None:
        return True
    return draft != base
