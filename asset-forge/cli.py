#!/usr/bin/env python3
"""Asset Forge CLI — create packs, entities, sprites without the UI."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from forge_lib.entities import default_entity, write_entity
from forge_lib.packs import create_pack, list_pack_ids, list_ruleset_ids, register_entity_in_manifest
from forge_lib.ruleset import scaffold_ruleset_variant
from forge_lib.sprites import generate_all_sprites
from forge_lib.validate import validate_all


def cmd_init_starter(_: argparse.Namespace) -> int:
    pack_id = "pack.starter"
    if pack_id in list_pack_ids():
        print(f"Pack already exists: {pack_id}")
        return 0
    create_pack(pack_id, "Starter slice", "pmr.v1", "Tutorial NPCs and props for PMR-Prime.")
    entities = [
        default_entity("npc.mentor", "Mentor", kind="npc", brain_seed=42),
        default_entity("prop.echo_gate", "Echo Gate", kind="prop", interactable=True, hp=1, stamina=0),
        default_entity("pickup.shard", "Shard", kind="pickup", interactable=True, hp=1, stamina=0),
    ]
    for ent in entities:
        path = write_entity(pack_id, ent)
        register_entity_in_manifest(pack_id, f"entities/{path.name}")
    generate_all_sprites(pack_id)
    print(f"Created {pack_id} under assets/packs/")
    return 0


def cmd_new_pack(args: argparse.Namespace) -> int:
    path = create_pack(args.pack_id, args.title, args.ruleset, args.description or "")
    print(f"Wrote {path}")
    return 0


def cmd_new_entity(args: argparse.Namespace) -> int:
    ent = default_entity(
        args.entity_id,
        args.name,
        kind=args.kind,
        interactable=not args.no_interact,
        hp=args.hp,
        stamina=args.stamina,
    )
    path = write_entity(args.pack, ent)
    register_entity_in_manifest(args.pack, f"entities/{path.name}")
    print(f"Wrote {path}")
    if args.sprites:
        generate_all_sprites(args.pack)
    return 0


def cmd_sprites(args: argparse.Namespace) -> int:
    paths = generate_all_sprites(args.pack)
    for p in paths:
        print(p)
    return 0


def cmd_validate(_: argparse.Namespace) -> int:
    errors = validate_all()
    if not errors:
        print("All checks passed.")
        return 0
    for line in errors:
        print(line, file=sys.stderr)
    return 1


def cmd_ruleset_fork(args: argparse.Namespace) -> int:
    path = scaffold_ruleset_variant(args.base, args.new_id, args.title, tightness_delta=args.tightness)
    print(f"Wrote {path}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Hades Engine Asset Forge")
    sub = parser.add_subparsers(dest="command", required=True)

    p_init = sub.add_parser("init-starter", help="Create pack.starter with sample entities")
    p_init.set_defaults(func=cmd_init_starter)

    p_pack = sub.add_parser("new-pack", help="Create an empty content pack")
    p_pack.add_argument("pack_id", help="e.g. pack.myzone")
    p_pack.add_argument("title")
    p_pack.add_argument("ruleset", help="ruleset id, e.g. pmr.v1")
    p_pack.add_argument("--description", default="")
    p_pack.set_defaults(func=cmd_new_pack)

    p_ent = sub.add_parser("new-entity", help="Add an entity JSON to a pack")
    p_ent.add_argument("pack", help="pack folder name, e.g. pack.starter")
    p_ent.add_argument("entity_id", help="e.g. npc.guard")
    p_ent.add_argument("name", help="display name")
    p_ent.add_argument("--kind", default="npc", choices=["npc", "prop", "pickup", "player_spawn"])
    p_ent.add_argument("--hp", type=float, default=100.0)
    p_ent.add_argument("--stamina", type=float, default=100.0)
    p_ent.add_argument("--no-interact", action="store_true")
    p_ent.add_argument("--sprites", action="store_true", help="Regenerate all pack sprites")
    p_ent.set_defaults(func=cmd_new_entity)

    p_spr = sub.add_parser("sprites", help="Generate placeholder PNG sprites for a pack")
    p_spr.add_argument("pack")
    p_spr.set_defaults(func=cmd_sprites)

    p_val = sub.add_parser("validate", help="Validate rulesets and all packs")
    p_val.set_defaults(func=cmd_validate)

    p_rs = sub.add_parser("ruleset-fork", help="Copy a ruleset JSON with a new id/title")
    p_rs.add_argument("base", help="base ruleset id without .json")
    p_rs.add_argument("new_id")
    p_rs.add_argument("title")
    p_rs.add_argument("--tightness", type=float, default=0.0)
    p_rs.set_defaults(func=cmd_ruleset_fork)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
