"""Curated ruleset paths for extension patch autocomplete."""

PATCH_PATH_CATALOG: list[tuple[str, str]] = [
    ("tightness", "Core simulation tightness 0–1"),
    ("dt_ms", "Tick duration in milliseconds"),
    ("motion.gravity", "World gravity"),
    ("motion.max_speed", "Max avatar speed"),
    ("motion.air_control", "Air control factor"),
    ("motion.blink", "Enable blink teleport (bool)"),
    ("motion.c_info_m_s", "Causality info speed cap (null to disable)"),
    ("conservation.items", "unique | clone-tax | unlimited"),
    ("conservation.clone_tax_entropy", "Clone tax when items=clone-tax"),
    ("death.unbind", "Unbind FWAU on death"),
    ("death.park_s", "Death park seconds"),
    ("death.rewind_s", "Rewind seconds"),
    ("psi.enabled", "Psi queries enabled"),
    ("psi.base_cost", "Psi base entropy cost"),
    ("sleep.delay_s", "LOD sleep delay"),
    ("sleep.kinematic_wake", "Kinematic wake flag"),
    ("handoff.allowed_targets", "JSON array of ruleset ids"),
    ("verbs.attack.enabled", "Attack verb on/off"),
    ("verbs.attack.damage", "Attack damage"),
    ("verbs.attack.range_m", "Attack range meters"),
    ("verbs.attack.stamina_cost", "Attack stamina cost"),
    ("verbs.attack.harm_entropy", "Attack harm entropy"),
    ("verbs.interact.enabled", "Interact verb on/off"),
    ("verbs.interact.range_m", "Interact range"),
    ("verbs.interact.item_prefix", "Echo item prefix string"),
    ("verbs.speak.enabled", "Speak verb on/off"),
    ("verbs.speak.range_m", "Speak range"),
    ("verbs.assist.enabled", "Assist verb on/off"),
    ("verbs.assist.range_m", "Assist range"),
    ("verbs.assist.heal_amount", "Assist heal amount"),
    ("verbs.assist.stamina_cost", "Assist stamina cost"),
    ("verbs.assist.aid_entropy", "Assist aid entropy"),
    ("verbs.assist.ai_practice", "AI practice flag"),
    ("crdt.props", "CRDT props model (e.g. or-set)"),
    ("crdt.presence", "CRDT presence model"),
    ("presentation.combat", "Full combat FX profile object (see mechanics/catalog/combat_fx_presets.json)"),
    ("presentation.combat.profile_id", "Active combat presentation preset id"),
    ("presentation.combat.hit_stop_ms", "Hit-stop duration on connect"),
    ("presentation.combat.screen_shake", "Screen shake intensity 0–1"),
]


def path_labels() -> list[str]:
    return [p for p, _ in PATCH_PATH_CATALOG]


def describe_path(path: str) -> str:
    for p, d in PATCH_PATH_CATALOG:
        if p == path:
            return d
    return "Custom path"
