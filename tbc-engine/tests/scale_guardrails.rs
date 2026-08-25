//! M8 scale and guardrail integration tests.

use tbc_engine::frame::{Frame, FrameSpec};
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::iuoc::IuocRegistry;
use tbc_engine::ledger::EntropyLedger;
use tbc_engine::netcode::IntentReject;
use tbc_engine::ruleset::Ruleset;
use tbc_engine::types::{FrameId, FwauId, Tick, Vec3};

fn test_frame() -> Frame {
  let ruleset = Ruleset::pmr_prime();
  Frame::new(FrameSpec {
    id: FrameId(1),
    name: "scale-test".into(),
    ruleset,
    genesis_hash: [0u8; 32],
  })
}

#[test]
fn pmr_200_entities_stays_within_budget() {
  let mut frame = test_frame();
  let mut ledger = EntropyLedger::new();

  for i in 0..200 {
    let pos = Vec3::new(i as f32 * 4.0, (i % 10) as f32 * 3.0, 0.0);
    let e = frame.world.spawn_ai_guy(&format!("scale-{}", i), pos, i as u64 + 1);
    frame.grid.insert_entity(e, pos, 1);
    if let Some(rec) = frame.world.get_mut(e) {
      rec.sleep.awake = true;
    }
  }

  let mut within_ticks = 0;
  for _ in 0..600 {
    frame.step_once(&mut ledger);
    if frame.island_mgr.profiler.within_budget {
      within_ticks += 1;
    }
  }

  let pct = within_ticks as f32 / 600.0;
  assert!(pct >= 0.95, "only {:.1}% ticks within budget", pct * 100.0);
  assert_eq!(frame.guardrails.stall_count, 0);
}

#[test]
fn intent_flood_rate_limited() {
  let mut frame = test_frame();
  let mut registry = IuocRegistry::new();
  let iuoc = registry.create_soul();
  let fwau = frame
    .bind_player(&mut registry, iuoc, Vec3::new(0.0, 0.0, 0.0))
    .expect("bind");

  let tick = frame.now();
  let mut accepted = 0;
  let mut rejected = 0;

  for i in 0..500 {
    let intent = Intent {
      fwau,
      tick,
      seq: i,
      verb: Verb::Move,
      payload: vec![0, 0, 0, 0, 0, 0, 0, 0],
      consent: None,
      checksum: 0,
    };
    match frame.submit_intent(intent) {
      Ok(()) => accepted += 1,
      Err(IntentReject::RateLimited) | Err(IntentReject::QueueFull) => rejected += 1,
      Err(e) => panic!("unexpected reject: {:?}", e),
    }
  }

  assert!(rejected > accepted, "expected rate limit to dominate flood");
  assert!(frame.guardrails.rate_limited > 0 || frame.guardrails.queue_full > 0);
}

#[test]
fn unknown_fwau_rejected() {
  let mut frame = test_frame();
  let intent = Intent {
    fwau: FwauId(999),
    tick: Tick(1),
    seq: 1,
    verb: Verb::Move,
    payload: vec![0, 0, 0, 0, 0, 0, 0, 0],
    consent: None,
    checksum: 0,
  };
  let err = frame.submit_intent(intent).unwrap_err();
  assert_eq!(err, IntentReject::UnknownFwau);
}
