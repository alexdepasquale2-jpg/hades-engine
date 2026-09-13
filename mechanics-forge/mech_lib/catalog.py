from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

Category = Literal[
    "core",
    "motion",
    "combat",
    "social",
    "economy",
    "death",
    "psi",
    "network",
    "world",
]


@dataclass(frozen=True)
class MechanicEntry:
    id: str
    category: Category
    name: str
    description: str
    ruleset_path: str | None
    intent_verb: str | None
    engine_implemented: bool


ENGINE_CATALOG: list[MechanicEntry] = [
    MechanicEntry(
        "core.tightness",
        "core",
        "Simulation tightness",
        "Entropy / determinism scalar for the frame.",
        "tightness",
        None,
        True,
    ),
    MechanicEntry(
        "core.dt_ms",
        "core",
        "Tick duration (ms)",
        "Simulation step size; drives physics and intent timing.",
        "dt_ms",
        None,
        True,
    ),
    MechanicEntry(
        "core.clock",
        "core",
        "Clock ratio",
        "Parent clock and n:k tick ratio.",
        "clock",
        None,
        True,
    ),
    MechanicEntry(
        "motion.envelope",
        "motion",
        "Movement envelope",
        "Gravity, max speed, air control, blink teleport, causality cap.",
        "motion",
        "Move",
        True,
    ),
    MechanicEntry(
        "motion.blink",
        "motion",
        "Blink teleport",
        "When enabled, Blink intent sets position directly.",
        "motion.blink",
        "Blink",
        True,
    ),
    MechanicEntry(
        "verb.attack",
        "combat",
        "Attack",
        "Damage, range, stamina cost, harm entropy.",
        "verbs.attack",
        "Attack",
        True,
    ),
    MechanicEntry(
        "verb.interact",
        "economy",
        "Interact / echoes",
        "Grant unique items with ruleset item prefix (echo, thought, dream).",
        "verbs.interact",
        "Interact",
        True,
    ),
    MechanicEntry(
        "verb.speak",
        "social",
        "Speak",
        "Proximity chat broadcast with range.",
        "verbs.speak",
        "Speak",
        True,
    ),
    MechanicEntry(
        "verb.assist",
        "social",
        "Assist / heal",
        "Heal allies with stamina cost, aid entropy, optional AI practice.",
        "verbs.assist",
        "Assist",
        True,
    ),
    MechanicEntry(
        "death.policy",
        "death",
        "Death & unbind",
        "Unbind FWAU on death, park/rewind timers.",
        "death",
        None,
        True,
    ),
    MechanicEntry(
        "psi.policy",
        "psi",
        "Psi scopes",
        "Past/future queries with base entropy cost.",
        "psi",
        "PsiQuery",
        True,
    ),
    MechanicEntry(
        "sleep.policy",
        "world",
        "Entity sleep",
        "LOD sleep delay and kinematic wake.",
        "sleep",
        None,
        True,
    ),
    MechanicEntry(
        "handoff.policy",
        "world",
        "Frame handoff",
        "Allowed ruleset targets for FWAU migration.",
        "handoff",
        None,
        True,
    ),
    MechanicEntry(
        "conservation",
        "economy",
        "Item conservation",
        "Unique items, clone tax, currency model.",
        "conservation",
        None,
        True,
    ),
    MechanicEntry(
        "crdt.props",
        "world",
        "CRDT props",
        "Optional replicated prop store (or-set / LWW).",
        "crdt",
        None,
        True,
    ),
    MechanicEntry(
        "intent.look",
        "network",
        "Look intent",
        "Netcode verb (not ruleset-tuned in JSON yet).",
        None,
        "Look",
        True,
    ),
    MechanicEntry(
        "intent.consent",
        "social",
        "Consent stamps",
        "Assist/speak consent wire for multiplayer.",
        None,
        "Consent",
        True,
    ),
    MechanicEntry(
        "intent.emote",
        "social",
        "Emote",
        "Emote verb channel.",
        None,
        "Emote",
        False,
    ),
    MechanicEntry(
        "shard.cross",
        "world",
        "Shard crossing",
        "Distributed handoff via RWW (engine feature).",
        None,
        None,
        True,
    ),
    MechanicEntry(
        "reincarnation",
        "death",
        "Reincarnation offers",
        "Planner-ranked respawn templates.",
        None,
        None,
        True,
    ),
    MechanicEntry(
        "presentation.combat",
        "combat",
        "Combat presentation profile",
        "Hit-stop, screen shake, particles, camera, audio, haptics, and satisfiers on attack connect (ruleset presentation.combat or extensions).",
        "presentation.combat",
        "Attack",
        False,
    ),
    MechanicEntry(
        "presentation.hit_stop",
        "combat",
        "Hit-stop / time freeze",
        "Brief time scale dip on hit for tactile confirmation.",
        "presentation.combat.hit_stop_ms",
        "Attack",
        False,
    ),
    MechanicEntry(
        "presentation.satisfiers",
        "combat",
        "Combat satisfiers",
        "Combo counters, parry rings, execute banners, entropy sparks, AoE rings, etc.",
        "presentation.combat.satisfiers",
        None,
        False,
    ),
    MechanicEntry(
        "extensions.combat_fx",
        "combat",
        "Combat FX extension packs",
        "24 apply-ready profiles under mechanics/extensions/custom.combat_fx_*.json plus master library.",
        None,
        None,
        False,
    ),
]


def categories() -> list[str]:
    return sorted({m.category for m in ENGINE_CATALOG})


def by_category(category: str | None) -> list[MechanicEntry]:
    if not category or category == "all":
        return list(ENGINE_CATALOG)
    return [m for m in ENGINE_CATALOG if m.category == category]
