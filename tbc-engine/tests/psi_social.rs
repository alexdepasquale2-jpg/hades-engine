//! M15 — psi scopes, Speak, and Consent pacts.

use tbc_engine::aum::AumCore;
use tbc_engine::types::{FwauId, IuocId, Vec3};

#[test]
fn psi_future_self_spend_budget() {
    let genesis = *blake3::hash(b"m15-psi-budget").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(4);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(0, iuoc, Vec3::ZERO).unwrap();
    let before = aum.iuoc.psi_budget_remaining(fwau);
    let res = aum.query_psi(0, fwau, iuoc, "FutureSelf");
    assert!(res.allowed);
    assert!(res.psi_budget_remaining < before);
}

#[test]
fn psi_past_shared_npmr_frame() {
    let genesis = *blake3::hash(b"m15-past-shared").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    let donor = aum.iuoc.create_soul();
    let listener = aum.iuoc.create_soul();
    let donor_fwau = aum
        .bind_player(2, donor, Vec3::new(10.0, 0.0, 0.0))
        .unwrap();
    aum.unbind_death(donor_fwau, 2);
    let listener_fwau = aum.bind_player(2, listener, Vec3::ZERO).unwrap();
    let res = aum.query_psi(2, listener_fwau, listener, "PastShared");
    assert!(res.allowed);
    assert!(!res.recall.is_empty());
}

#[test]
fn speak_heard_nearby_ai() {
    let genesis = *blake3::hash(b"m15-speak").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(6);
    let iuoc = aum.iuoc.create_soul();
    // Demo AI spawn near shard-00 center (~x=-175); bind beside them for range check.
    let fwau = aum
        .bind_player(0, iuoc, Vec3::new(-175.0, 0.0, 0.0))
        .unwrap();
    let res = aum.speak(0, fwau, "The seam remembers.", None);
    assert!(res.heard);
    assert!(res.listeners >= 1);
    assert!(!aum.frames[0].recent_speaks.is_empty());
}

#[test]
fn consent_pact_grants_assist() {
    let genesis = *blake3::hash(b"m15-consent").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    let from = aum.iuoc.create_soul();
    let target = aum.iuoc.create_soul();
    let pact = aum
        .grant_consent(from, target, "assist", 10_000)
        .expect("grant");
    assert!(pact.contains("assist"));
    assert!(aum.has_consent(from, target, "assist"));
}

#[test]
fn speak_disabled_when_ruleset_off() {
    let genesis = *blake3::hash(b"m15-speak-off").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spec.ruleset.verbs.speak.enabled = false;
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(0, iuoc, Vec3::ZERO).unwrap();
    let res = aum.speak(0, fwau, "muted", None);
    assert!(!res.heard);
    assert!(res.message.contains("disabled"));
}
