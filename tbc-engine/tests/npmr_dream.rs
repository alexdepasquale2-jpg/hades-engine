//! M14 — handoff policy enforcement and NPMR Dream ruleset.

use tbc_engine::aum::AumCore;
use tbc_engine::ruleset::RulesetRegistry;
use tbc_engine::types::Vec3;

#[test]
fn handoff_policy_blocks_invalid_target() {
    let genesis = *blake3::hash(b"handoff-policy").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(2);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum
        .bind_player(0, iuoc, Vec3::new(-80.0, 0.0, 0.0))
        .unwrap();
    let err = aum.handoff_fwau(fwau, 0, 3).unwrap_err();
    assert!(err.contains("forbids handoff"));
}

#[test]
fn handoff_academy_to_dream_allowed() {
    let genesis = *blake3::hash(b"handoff-dream").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[2].spawn_demo_world(2);
    aum.frames[3].spawn_demo_world(2);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(2, iuoc, Vec3::ZERO).unwrap();
    assert!(aum.handoff_fwau(fwau, 2, 3).is_ok());
}

#[test]
fn npmr_dream_ruleset_loads() {
    let path = format!("{}/../rulesets", env!("CARGO_MANIFEST_DIR"));
    let reg = RulesetRegistry::load_dir(std::path::Path::new(&path)).expect("load");
    let dream = reg.get("npmr.dream.v1").expect("dream");
    assert!(dream.crdt.is_some());
    assert!(dream.verbs.interact.enabled);
}

#[test]
fn cluster_has_four_frames() {
    let aum = AumCore::boot_cluster(*blake3::hash(b"four-frames").as_bytes());
    assert_eq!(aum.frames.len(), 4);
    assert_eq!(aum.frames[3].spec.ruleset.id, "npmr.dream.v1");
}

#[test]
fn interact_places_crdt_prop_in_npmr() {
    let genesis = *blake3::hash(b"crdt-prop").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[2]
        .world
        .spawn_ai_guy("Near", Vec3::new(2.0, 0.0, 0.0), 99);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(2, iuoc, Vec3::ZERO).unwrap();
    let res = aum.interact(2, fwau);
    assert!(res.hit);
    assert_eq!(aum.frames[2].prop_store.len(), 1);
}
