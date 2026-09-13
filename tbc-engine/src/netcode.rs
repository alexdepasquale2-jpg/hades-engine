use crate::intent::Intent;
use crate::types::{Entity, FwauId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SNAPSHOT_RING: usize = 8;
pub const MAX_AHEAD_TICKS: u64 = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityPose {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug)]
pub struct TickSnapshot {
    pub tick: Tick,
    pub poses: Vec<EntityPose>,
    pub checksum: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Correction {
    pub from_tick: u64,
    pub checksums: HashMap<u64, u32>,
    pub poses: Vec<EntityPose>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentReject {
    TooOld,
    TooFarAhead,
    SpeedHack,
    UnknownFwau,
    RateLimited,
    QueueFull,
}

/// Authoritative rewind-replay netcode state (spec §7).
pub struct NetcodeState {
    pub ring: Vec<Option<TickSnapshot>>,
    pub pending: HashMap<FwauId, HashMap<u64, Intent>>,
    pub auth_tick: Tick,
    pub corrections: Vec<Correction>,
    pub rejects: Vec<(FwauId, IntentReject)>,
}

impl NetcodeState {
    pub fn new() -> Self {
        Self {
            ring: vec![None; SNAPSHOT_RING],
            pending: HashMap::new(),
            auth_tick: Tick(0),
            corrections: Vec::new(),
            rejects: Vec::new(),
        }
    }

    pub fn ring_index(tick: u64) -> usize {
        tick as usize % SNAPSHOT_RING
    }

    pub fn checksum(poses: &[EntityPose]) -> u32 {
        let mut hash = 0u32;
        for p in poses {
            hash ^= p.entity.index;
            hash = hash.wrapping_add((p.x * 1000.0) as u32);
            hash = hash.wrapping_add((p.y * 1000.0) as u32);
        }
        hash
    }

    pub fn store_snapshot(&mut self, tick: Tick, poses: Vec<EntityPose>) {
        let checksum = Self::checksum(&poses);
        let snap = TickSnapshot {
            tick,
            poses,
            checksum,
        };
        self.ring[Self::ring_index(tick.0)] = Some(snap);
        self.auth_tick = tick;
    }

    /// Reuse `poses` buffer across ticks (caller clears or we take empty vec back).
    pub fn store_snapshot_reuse(&mut self, tick: Tick, poses: &mut Vec<EntityPose>) {
        let checksum = Self::checksum(poses);
        let snap = TickSnapshot {
            tick,
            poses: std::mem::take(poses),
            checksum,
        };
        self.ring[Self::ring_index(tick.0)] = Some(snap);
        self.auth_tick = tick;
    }

    pub fn snapshot_at(&self, tick: Tick) -> Option<&TickSnapshot> {
        self.ring[Self::ring_index(tick.0)]
            .as_ref()
            .filter(|s| s.tick == tick)
    }

    /// Accept or buffer an intent. Late intents trigger rewind flag.
    pub fn accept_intent(
        &mut self,
        intent: Intent,
        pending_cap: usize,
    ) -> Result<bool, IntentReject> {
        let fwau = intent.fwau;
        let t = intent.tick.0;
        let auth = self.auth_tick.0;

        if t + (SNAPSHOT_RING as u64) < auth {
            return Err(IntentReject::TooOld);
        }
        if t > auth + MAX_AHEAD_TICKS {
            return Err(IntentReject::TooFarAhead);
        }

        let bucket = self.pending.entry(fwau).or_default();
        if bucket.len() >= pending_cap {
            return Err(IntentReject::QueueFull);
        }
        bucket.insert(t, intent);

        Ok(t < auth)
    }

    pub fn pending_count(&self, fwau: FwauId) -> usize {
        self.pending.get(&fwau).map(|b| b.len()).unwrap_or(0)
    }

    pub fn pending_for_tick(&self, tick: Tick) -> Vec<Intent> {
        let mut out = Vec::new();
        self.pending_for_tick_into(tick, &mut out);
        out
    }

    pub fn pending_for_tick_into(&self, tick: Tick, out: &mut Vec<Intent>) {
        out.clear();
        let t = tick.0;
        for bucket in self.pending.values() {
            if let Some(i) = bucket.get(&t) {
                out.push(i.clone());
            }
        }
    }

    pub fn clear_pending_through(&mut self, tick: Tick) {
        for bucket in self.pending.values_mut() {
            bucket.retain(|t, _| *t > tick.0);
        }
    }

    pub fn push_correction(&mut self, correction: Correction) {
        self.corrections.push(correction);
    }

    pub fn drain_corrections(&mut self) -> Vec<Correction> {
        std::mem::take(&mut self.corrections)
    }

    pub fn record_reject(&mut self, fwau: FwauId, reason: IntentReject) {
        self.rejects.push((fwau, reason));
    }

    pub fn drain_rejects(&mut self) -> Vec<(FwauId, IntentReject)> {
        std::mem::take(&mut self.rejects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Verb;

    fn dummy_intent(fwau: FwauId, tick: u64) -> Intent {
        Intent {
            fwau,
            tick: Tick(tick),
            seq: 1,
            verb: Verb::Move,
            payload: vec![0, 0, 0, 0, 0, 0, 0, 0],
            consent: None,
            checksum: 0,
        }
    }

    #[test]
    fn late_intent_signals_rewind() {
        let mut net = NetcodeState::new();
        net.auth_tick = Tick(10);
        net.store_snapshot(Tick(10), vec![]);

        let fwau = FwauId(1);
        let needs_rewind = net
            .accept_intent(dummy_intent(fwau, 8), 64)
            .expect("should accept");
        assert!(needs_rewind);
    }

    #[test]
    fn too_old_rejected() {
        let mut net = NetcodeState::new();
        net.auth_tick = Tick(20);
        let err = net
            .accept_intent(dummy_intent(FwauId(1), 1), 64)
            .unwrap_err();
        assert_eq!(err, IntentReject::TooOld);
    }
}
