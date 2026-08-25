use crate::types::{IuocId, QualityBand, QualityScalar, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

const ETA_AID: f32 = 0.040;
const ETA_HARM: f32 = 0.060;
const ETA_EGO: f32 = 0.030;
const ETA_COERCE: f32 = 0.080;
const SIGMA: f32 = 0.010;
const CLAMP: f32 = 0.150;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedAction {
  pub id: u128,
  pub aid: f32,
  pub harm: f32,
  pub ego: f32,
  pub coerce: f32,
  pub perf: f32,
}

#[derive(Clone, Debug)]
struct PendingDelta {
  actor: IuocId,
  delta: f32,
  action_id: u128,
}

/// Private, delayed, noisy entropy ledger — the character sheet.
pub struct EntropyLedger {
  pub scores: HashMap<IuocId, QualityScalar>,
  pending: BTreeMap<(Tick, u128), PendingDelta>,
}

impl EntropyLedger {
  pub fn new() -> Self {
    Self {
      scores: HashMap::new(),
      pending: BTreeMap::new(),
    }
  }

  pub fn get(&self, id: IuocId) -> QualityScalar {
    self.scores.get(&id).copied().unwrap_or(QualityScalar::INITIAL)
  }

  pub fn band(&self, id: IuocId) -> QualityBand {
    self.get(id).band()
  }

  pub fn enqueue_consequence(&mut self, actor: IuocId, action: &ResolvedAction, due_tick: Tick) {
    let raw = -ETA_AID * action.aid * (1.0 - action.perf)
      + ETA_HARM * action.harm
      + ETA_EGO * action.ego
      + ETA_COERCE * action.coerce;

    let noise = (rand::random::<f32>() - 0.5) * SIGMA * 2.0;
    let noisy = (raw + noise).clamp(-CLAMP, CLAMP);

  self.enqueue(actor, noisy, due_tick, action.id);
  }

  pub fn enqueue(&mut self, actor: IuocId, delta: f32, due: Tick, action_id: u128) {
    self.pending.insert(
      (due, action_id),
      PendingDelta {
        actor,
        delta,
        action_id,
      },
    );
  }

  pub fn flush_due(&mut self, now: Tick) -> Vec<(IuocId, QualityScalar, QualityBand)> {
    let mut applied = Vec::new();
    let due_keys: Vec<_> = self
      .pending
      .keys()
      .filter(|(t, _)| *t <= now)
      .copied()
      .collect();

    for key in due_keys {
      if let Some(ev) = self.pending.remove(&key) {
        let current = self.get(ev.actor);
        let new_s = QualityScalar::clamped(current.value() + ev.delta);
        self.scores.insert(ev.actor, new_s);
        applied.push((ev.actor, new_s, new_s.band()));
      }
    }
    applied
  }

  pub fn pending_count(&self) -> usize {
    self.pending.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn harm_raises_entropy() {
    let mut ledger = EntropyLedger::new();
    let actor = IuocId(1);
    ledger.enqueue_consequence(
      actor,
      &ResolvedAction {
        id: 1,
        aid: 0.0,
        harm: 1.0,
        ego: 0.0,
        coerce: 0.0,
        perf: 0.0,
      },
      Tick(10),
    );
    let changes = ledger.flush_due(Tick(10));
    assert_eq!(changes.len(), 1);
    assert!(changes[0].1.value() > QualityScalar::INITIAL.value());
  }

  #[test]
  fn aid_lowers_entropy() {
    let mut ledger = EntropyLedger::new();
    let actor = IuocId(2);
    ledger.enqueue_consequence(
      actor,
      &ResolvedAction {
        id: 2,
        aid: 1.0,
        harm: 0.0,
        ego: 0.0,
        coerce: 0.0,
        perf: 0.0,
      },
      Tick(5),
    );
    let changes = ledger.flush_due(Tick(5));
    assert!(changes[0].1.value() < QualityScalar::INITIAL.value());
  }
}
