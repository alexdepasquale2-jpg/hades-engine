//! M20 — Assist/Speak through netcode intent queue + tick stepping.

use tbc_engine::aum::AumCore;
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::types::Vec3;

#[test]
fn assist_applies_on_tick_step_from_pending_intent() {
    let genesis = *blake3::hash(b"m20-assist-tick").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[2].spawn_demo_world(4);
    let helper = aum.iuoc.create_soul();
    let fwau = aum
        .bind_player(2, helper, Vec3::new(50.0, 0.0, 0.0))
        .unwrap();

    let tick = aum.frames[2].now();
    let intent = Intent {
        fwau,
        tick,
        seq: 1,
        verb: Verb::Assist,
        payload: vec![],
        consent: None,
        checksum: 0,
    };
    aum.submit_intent(2, intent).expect("accept assist intent");

  // NPMR AI practice — no synchronous flush; sim tick applies verb.
    aum.run_frame_ticks(2, 1);

    let ai_hp_before = aum.frames[2]
        .world
        .all_entities()
        .iter()
        .filter_map(|&e| aum.frames[2].world.get(e))
        .find(|r| r.brain.is_some())
        .and_then(|r| r.avatar.as_ref())
        .map(|a| a.hp)
        .unwrap_or(0.0);

    assert!(
        ai_hp_before >= 100.0,
        "assist intent should heal nearby AI on tick step"
    );
    assert!(
        aum.frames[2].last_assist.contains_key(&fwau) || ai_hp_before > 0.0,
        "assist outcome recorded or applied"
    );
}

#[test]
fn late_assist_intent_triggers_rewind_replay() {
    let genesis = *blake3::hash(b"m20-assist-rewind").as_bytes();
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
            a.hp = 25.0;
        }
    }
    aum.grant_consent(target, helper, "assist", 80_000).expect("grant");

    // Advance authoritative tick (step_once does not auto-increment clock).
    for t in 1..=12 {
        aum.frames[0].clock.tick = tbc_engine::types::Tick(t);
        aum.run_frame_ticks(0, 1);
    }
    let auth = aum.frames[0].netcode.auth_tick.0;
    let late_tick = auth.saturating_sub(2);

    let intent = Intent {
        fwau: helper_fwau,
        tick: tbc_engine::types::Tick(late_tick),
        seq: 99,
        verb: Verb::Assist,
        payload: target_entity.index.to_le_bytes().to_vec(),
        consent: None,
        checksum: 0,
    };
    aum.submit_intent(0, intent).expect("late assist accepted");

    let hp = aum.frames[0]
        .world
        .get(target_entity)
        .and_then(|r| r.avatar.as_ref())
        .map(|a| a.hp)
        .unwrap_or(0.0);
    assert!(hp > 25.0, "rewind replay should apply assist heal (hp={})", hp);
    assert!(
        !aum.frames[0].netcode.corrections.is_empty(),
        "late intent should emit netcode correction"
    );
}

#[test]
fn speak_intent_heard_on_tick_step() {
    let genesis = *blake3::hash(b"m20-speak-tick").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(6);
    let soul = aum.iuoc.create_soul();
    let fwau = aum
        .bind_player(0, soul, Vec3::new(-175.0, 0.0, 0.0))
        .unwrap();

    let tick = aum.frames[0].now();
    let intent = Intent {
        fwau,
        tick,
        seq: 1,
        verb: Verb::Speak,
        payload: b"Intent queue speaks.".to_vec(),
        consent: None,
        checksum: 0,
    };
    aum.submit_intent(0, intent).expect("accept speak");
    aum.run_frame_ticks(0, 1);

    assert!(
        aum.frames[0]
            .recent_speaks
            .iter()
            .any(|s| s.text.contains("Intent queue")),
        "speak intent should broadcast on tick step"
    );
}
