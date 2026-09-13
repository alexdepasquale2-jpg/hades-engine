from __future__ import annotations

from copy import deepcopy
from typing import Any

from mech_lib.ruleset_io import apply_patches, ensure_verb_defaults


def sync_draft_from_widgets(
    draft: dict[str, Any],
    *,
    ruleset_id: str,
    title: str,
    tightness: float,
    dt_ms: int,
    gravity: float,
    max_speed: float,
    air_control: float,
    blink: bool,
    c_info: float | None,
    cons_items: str,
    cons_currency: str,
    clone_tax: float | None,
    death_unbind: bool,
    death_park: int,
    death_rewind: int,
    psi_enabled: bool,
    psi_cost: float,
    psi_scopes: list[str],
    sleep_delay: float,
    sleep_kinematic: bool,
    handoff_targets: str,
    atk_enabled: bool,
    atk_damage: float,
    atk_range: float,
    atk_stamina: float,
    atk_entropy: float,
    int_enabled: bool,
    int_range: float,
    int_prefix: str,
    spk_enabled: bool,
    spk_range: float,
    ast_enabled: bool,
    ast_range: float,
    ast_stamina: float,
    ast_heal: float,
    ast_entropy: float,
    ast_ai: bool,
    crdt_enabled: bool,
    crdt_props: str,
    crdt_presence: str,
) -> dict[str, Any]:
    patches: dict[str, Any] = {
        "id": ruleset_id.strip(),
        "title": title.strip(),
        "tightness": round(tightness, 3),
        "dt_ms": int(dt_ms),
        "motion.gravity": gravity,
        "motion.max_speed": max_speed,
        "motion.air_control": air_control,
        "motion.blink": blink,
        "motion.c_info_m_s": c_info,
        "conservation.items": cons_items,
        "death.unbind": death_unbind,
        "death.park_s": death_park,
        "death.rewind_s": death_rewind,
        "psi.enabled": psi_enabled,
        "psi.base_cost": psi_cost,
        "psi.scopes": list(psi_scopes),
        "sleep.delay_s": sleep_delay,
        "sleep.kinematic_wake": sleep_kinematic,
        "verbs.attack.enabled": atk_enabled,
        "verbs.attack.damage": atk_damage,
        "verbs.attack.range_m": atk_range,
        "verbs.attack.stamina_cost": atk_stamina,
        "verbs.attack.harm_entropy": atk_entropy,
        "verbs.interact.enabled": int_enabled,
        "verbs.interact.range_m": int_range,
        "verbs.interact.item_prefix": int_prefix.strip() or "echo",
        "verbs.speak.enabled": spk_enabled,
        "verbs.speak.range_m": spk_range,
        "verbs.assist.enabled": ast_enabled,
        "verbs.assist.range_m": ast_range,
        "verbs.assist.stamina_cost": ast_stamina,
        "verbs.assist.heal_amount": ast_heal,
        "verbs.assist.aid_entropy": ast_entropy,
        "verbs.assist.ai_practice": ast_ai,
    }
    targets = [t.strip() for t in handoff_targets.split(",") if t.strip()]
    patches["handoff.allowed_targets"] = targets
    if cons_currency.strip():
        patches["conservation.currency"] = cons_currency.strip()
    if clone_tax is not None and cons_items == "clone-tax":
        patches["conservation.clone_tax_entropy"] = clone_tax
    out = apply_patches(draft, patches)
    if crdt_enabled:
        out["crdt"] = {"props": crdt_props, "presence": crdt_presence or None}
    else:
        out.pop("crdt", None)
    out.setdefault("clock", {"parent": None, "n": 1, "k": 1})
    out["verbs"] = ensure_verb_defaults(out.get("verbs", {}))
    return out


def read_widget_defaults(draft: dict[str, Any]) -> dict[str, Any]:
    motion = draft.get("motion", {})
    cons = draft.get("conservation", {})
    death = draft.get("death", {})
    psi = draft.get("psi", {})
    sleep = draft.get("sleep", {})
    handoff = draft.get("handoff", {})
    verbs = ensure_verb_defaults(draft.get("verbs", {}))
    atk = verbs["attack"]
    intr = verbs["interact"]
    spk = verbs["speak"]
    ast = verbs["assist"]
    crdt = draft.get("crdt") or {}
    return {
        "ruleset_id": draft.get("id", "pmr.custom.v1"),
        "title": draft.get("title", "Custom"),
        "tightness": float(draft.get("tightness", 0.9)),
        "dt_ms": int(draft.get("dt_ms", 50)),
        "gravity": float(motion.get("gravity", 9.8)),
        "max_speed": float(motion.get("max_speed", 7.0)),
        "air_control": float(motion.get("air_control", 0.35)),
        "blink": bool(motion.get("blink", False)),
        "c_info": motion.get("c_info_m_s"),
        "cons_items": cons.get("items", "unique"),
        "cons_currency": cons.get("currency") or "",
        "clone_tax": cons.get("clone_tax_entropy"),
        "death_unbind": bool(death.get("unbind", True)),
        "death_park": int(death.get("park_s", 0)),
        "death_rewind": int(death.get("rewind_s", 0)),
        "psi_enabled": bool(psi.get("enabled", True)),
        "psi_cost": float(psi.get("base_cost", 0.04)),
        "psi_scopes": list(psi.get("scopes", ["PastOwn"])),
        "sleep_delay": float(sleep.get("delay_s", 2.0)),
        "sleep_kinematic": bool(sleep.get("kinematic_wake", True)),
        "handoff_targets": ", ".join(handoff.get("allowed_targets", [])),
        "atk_enabled": bool(atk.get("enabled", True)),
        "atk_damage": float(atk.get("damage", 34)),
        "atk_range": float(atk.get("range_m", 8)),
        "atk_stamina": float(atk.get("stamina_cost", 12)),
        "atk_entropy": float(atk.get("harm_entropy", 0.5)),
        "int_enabled": bool(intr.get("enabled", True)),
        "int_range": float(intr.get("range_m", 4)),
        "int_prefix": str(intr.get("item_prefix", "echo")),
        "spk_enabled": bool(spk.get("enabled", True)),
        "spk_range": float(spk.get("range_m", 24)),
        "ast_enabled": bool(ast.get("enabled", False)),
        "ast_range": float(ast.get("range_m", 6)),
        "ast_stamina": float(ast.get("stamina_cost", 8)),
        "ast_heal": float(ast.get("heal_amount", 25)),
        "ast_entropy": float(ast.get("aid_entropy", 0.35)),
        "ast_ai": bool(ast.get("ai_practice", False)),
        "crdt_enabled": bool(crdt),
        "crdt_props": str(crdt.get("props", "or-set")),
        "crdt_presence": str(crdt.get("presence") or ""),
    }


def summary_metrics(draft: dict[str, Any]) -> dict[str, str]:
    verbs = ensure_verb_defaults(draft.get("verbs", {}))
    on = sum(1 for k in ("attack", "interact", "speak", "assist") if verbs[k].get("enabled"))
    motion = draft.get("motion", {})
    return {
        "Ruleset": draft.get("id", "?"),
        "Title": draft.get("title", "?"),
        "Tick": f"{draft.get('dt_ms', '?')} ms",
        "Tightness": f"{draft.get('tightness', 0):.2f}",
        "Max speed": f"{motion.get('max_speed', '?')}",
        "Verbs on": f"{on}/4",
        "Blink": "yes" if motion.get("blink") else "no",
        "Psi": "on" if draft.get("psi", {}).get("enabled") else "off",
    }


def apply_json_merge(draft: dict[str, Any], raw: str) -> tuple[dict[str, Any], str | None]:
    import json

    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as exc:
        return draft, str(exc)
    if not isinstance(parsed, dict):
        return draft, "Root must be a JSON object"
    merged = deepcopy(draft)
    merged.update(parsed)
    return merged, None
