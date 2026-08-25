//! M18 — consent-stamped assist/speak on wire and HTTP.

use tbc_engine::aum::AumCore;
use tbc_engine::consent_wire::{consent_from_payload, WireConsentStamp};
use tbc_engine::transport::{wire_to_intent, WireMessage};
use tbc_engine::types::Vec3;

#[test]
fn wire_intent_carries_consent_stamp() {
    let stamp = WireConsentStamp {
        target: 42,
        scope: "assist".into(),
        expires_tick: 99_999,
    };
    let msg = WireMessage::assist(1, Some(3), Some(&stamp));
    let intent = wire_to_intent(&msg, 7).expect("intent");
    assert!(intent.consent.is_some());
    assert_eq!(intent.consent.as_ref().unwrap().scope, "assist");
}

#[test]
fn assist_accepts_valid_wire_stamp() {
    let genesis = *blake3::hash(b"m18-assist-stamp").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(4);
    let target = aum.iuoc.create_soul();
    let helper = aum.iuoc.create_soul();
    let target_fwau = aum
        .bind_player(0, target, Vec3::new(-175.0, 0.0, 0.0))
        .unwrap();
    let helper_fwau = aum
        .bind_player(0, helper, Vec3::new(-174.0, 0.0, 0.0))
        .unwrap();
    let target_entity = aum.frames[0]
        .find_avatar_for_fwau(target_fwau)
        .expect("target");
    if let Some(rec) = aum.frames[0].world.get_mut(target_entity) {
        if let Some(a) = &mut rec.avatar {
            a.hp = 30.0;
        }
    }
    let expires = aum.frames[0].now().0 + 80_000;
    aum.grant_consent(target, helper, "assist", 80_000)
        .expect("grant");
    let payload = WireMessage::assist(
        helper_fwau.0,
        Some(target_entity.index),
        Some(&WireConsentStamp::from_grant(
            target, helper, "assist", expires,
        )),
    );
    let stamp = consent_from_payload(&payload.payload).expect("stamp");
    let res = aum.assist(0, helper_fwau, Some(target_entity), Some(stamp));
    assert!(res.hit);
    assert!(res.consent_verified);
}
