use crate::beam::ObservationResult;
use crate::guardrails::{GuardrailConfig, GuardrailReport, GuardrailState};
use crate::clock::DeltaTClock;
use crate::ecs::World;
use crate::grid::HierGrid;
use crate::islands::IslandManager;
use crate::intent::{Intent, IntentQueue, Verb};
use crate::iuoc::{EntityRef, IuocRegistry};
use crate::ledger::{EntropyLedger, ResolvedAction};
use crate::netcode::{Correction, EntityPose, IntentReject, NetcodeState};
use crate::ruleset::Ruleset;
use crate::shard::ShardBounds;
use crate::types::{
  Entity, FwauId, FrameId, IslandId, IuocId, Tick, Vec3,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct FrameSpec {
  pub id: FrameId,
  pub name: String,
  pub ruleset: Ruleset,
  pub genesis_hash: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
  pub entity: Entity,
  pub name: String,
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub hp: Option<f32>,
  pub stamina: Option<f32>,
  pub inventory: Option<Vec<String>>,
  pub is_player: bool,
  pub is_ai: bool,
  pub awake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameSnapshot {
  pub frame_id: u32,
  pub frame_name: String,
  pub ruleset_id: String,
  pub tick: u64,
  pub entities: Vec<EntitySnapshot>,
  pub fwau_count: usize,
  pub ai_count: usize,
  pub corrections: Vec<Correction>,
  pub rejects: Vec<RejectNotice>,
  pub island_profiler: Option<crate::islands::IslandProfiler>,
  pub shard_id: Option<u32>,
  pub guardrails: Option<GuardrailReport>,
  pub props: Vec<crate::crdt_props::CrdtProp>,
}

impl FrameSnapshot {
  /// JSON-safe snapshot (u128 IDs as strings for wire transport).
  pub fn to_json_value(&self) -> serde_json::Value {
    serde_json::json!({
      "frame_id": self.frame_id,
      "frame_name": self.frame_name,
      "ruleset_id": self.ruleset_id,
      "tick": self.tick,
      "entities": self.entities,
      "fwau_count": self.fwau_count,
      "ai_count": self.ai_count,
      "corrections": self.corrections,
      "rejects": self.rejects.iter().map(|r| serde_json::json!({
        "fwau": r.fwau.to_string(),
        "reason": r.reason,
      })).collect::<Vec<_>>(),
      "island_profiler": self.island_profiler,
      "shard_id": self.shard_id,
      "guardrails": self.guardrails,
      "props": self.props,
    })
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RejectNotice {
  pub fwau: u128,
  pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationPacket {
  pub tick: u64,
  pub entities: Vec<EntitySnapshot>,
  pub band_changes: Vec<BandChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BandChange {
  pub iuoc: u128,
  pub band: String,
  pub message: String,
}

/// TBC frame — authoritative simulation shard.
pub struct Frame {
  pub spec: FrameSpec,
  pub clock: DeltaTClock,
  pub world: World,
  pub grid: HierGrid,
  pub island_mgr: IslandManager,
  pub intent_queues: HashMap<FwauId, IntentQueue>,
  pub netcode: NetcodeState,
  pub shard_bounds: Option<ShardBounds>,
  pub pending_shard_handoffs: Vec<(FwauId, u32)>,
  pub pending_kills: Vec<(Entity, FwauId)>,
  pub rng_seed: u64,
  pub guardrails: GuardrailState,
  pub prop_store: crate::crdt_props::PropStore,
  sleep_delay_ticks: u64,
  player_last_pos: HashMap<FwauId, Vec3>,
  was_awake: HashSet<Entity>,
}

impl Frame {
  pub fn new(spec: FrameSpec) -> Self {
    Self::new_with_shard(spec, None)
  }

  pub fn new_with_shard(spec: FrameSpec, shard_bounds: Option<ShardBounds>) -> Self {
    let dt_ms = spec.ruleset.dt_ms as u64;
    let sleep_delay_ticks = (spec.ruleset.sleep.delay_s * 1000.0 / dt_ms as f32) as u64;
    let rng_seed = 0xA00 + shard_bounds.as_ref().map(|s| s.id as u64).unwrap_or(0);
    let guard_config = GuardrailConfig::for_dt_ms(spec.ruleset.dt_ms);
    Self {
      clock: DeltaTClock::new(dt_ms),
      spec,
      world: World::new(),
      grid: HierGrid::new(),
      island_mgr: IslandManager::new(rng_seed),
      intent_queues: HashMap::new(),
      netcode: NetcodeState::new(),
      shard_bounds,
      pending_shard_handoffs: Vec::new(),
      pending_kills: Vec::new(),
      rng_seed,
      guardrails: GuardrailState::new(guard_config),
      prop_store: crate::crdt_props::PropStore::new(),
      sleep_delay_ticks,
      player_last_pos: HashMap::new(),
      was_awake: HashSet::new(),
    }
  }

  pub fn now(&self) -> Tick {
    self.clock.now()
  }

  pub fn spawn_demo_world(&mut self, ai_count: usize) {
    for i in 0..ai_count {
      let angle = (i as f32) * 0.5;
      let base_x = self.shard_bounds.as_ref().map(|s| (s.min_x + s.max_x) / 2.0).unwrap_or(0.0);
      let pos = Vec3::new(
        base_x + (angle.cos() * 50.0) + (i as f32 * 3.0),
        angle.sin() * 50.0,
        0.0,
      );
      let e = self.world.spawn_ai_guy(&format!("AI-Guy-{}", i), pos, i as u64 + 1);
      self.grid.insert_entity(e, pos, 1);
    }
  }

  pub fn bind_player(
    &mut self,
    registry: &mut IuocRegistry,
    iuoc: IuocId,
    pos: Vec3,
  ) -> Result<FwauId, String> {
    let fwau_id = FwauId(ulid::Ulid::new().0);
    let entity = self.world.spawn("Player");
    self.world.bind_avatar(entity, fwau_id, iuoc);
    if let Some(rec) = self.world.get_mut(entity) {
      rec.transform.position = pos;
      rec.dirty = true;
    }
    self.grid.insert_entity(entity, pos, 1);
    self.grid.subscribe_fwau(fwau_id, pos, 1);
    self.intent_queues.insert(fwau_id, IntentQueue::with_cap(64));

    let avatar_ref = EntityRef {
      index: entity.index,
      generation: entity.generation,
    };
    registry
      .bind_fwau(iuoc, fwau_id, avatar_ref, self.now())
      .map_err(|e| format!("{:?}", e))?;

    Ok(fwau_id)
  }

  pub fn submit_intent(&mut self, intent: Intent) -> Result<(), IntentReject> {
    if !self.intent_queues.contains_key(&intent.fwau) {
      self.netcode.record_reject(intent.fwau, IntentReject::UnknownFwau);
      return Err(IntentReject::UnknownFwau);
    }

    if !self.guardrails.allow_intent(intent.fwau) {
      self.netcode.record_reject(intent.fwau, IntentReject::RateLimited);
      return Err(IntentReject::RateLimited);
    }

    if intent.verb == Verb::Move && intent.payload.len() >= 8 {
      let dx = f32::from_le_bytes(intent.payload[0..4].try_into().unwrap());
      let dy = f32::from_le_bytes(intent.payload[4..8].try_into().unwrap());
      if dx.abs() > 1.05 || dy.abs() > 1.05 {
        self.netcode.record_reject(intent.fwau, IntentReject::SpeedHack);
        return Err(IntentReject::SpeedHack);
      }
    }

    let pending_cap = self.guardrails.config.pending_intent_cap;
    let needs_rewind = match self.netcode.accept_intent(intent.clone(), pending_cap) {
      Ok(r) => r,
      Err(e) => {
        if e == IntentReject::QueueFull {
          self.guardrails.record_queue_full();
        }
        self.netcode.record_reject(intent.fwau, e.clone());
        return Err(e);
      }
    };

    if needs_rewind {
      if self.guardrails.allow_rewind() {
        self.rewind_replay(intent.tick);
      }
    }
    Ok(())
  }

  pub fn unbind_fwau(&mut self, fwau: FwauId) {
    if let Some(entity) = self.find_avatar_for_fwau(fwau) {
      self.grid.remove_entity(entity);
      self.world.despawn(entity);
    }
    self.intent_queues.remove(&fwau);
    self.player_last_pos.remove(&fwau);
  }

  fn collect_poses(&self) -> Vec<EntityPose> {
    self
      .world
      .all_entities()
      .iter()
      .filter_map(|&e| {
        self.world.get(e).map(|rec| EntityPose {
          entity: rec.entity,
          x: rec.transform.position.x,
          y: rec.transform.position.y,
          z: rec.transform.position.z,
        })
      })
      .collect()
  }

  fn apply_poses(&mut self, poses: &[EntityPose]) {
    for pose in poses {
      if let Some(rec) = self.world.get_mut(pose.entity) {
        rec.transform.position = Vec3::new(pose.x, pose.y, pose.z);
        rec.dirty = true;
      }
    }
  }

  fn store_tick_snapshot(&mut self, tick: Tick) {
    let poses = self.collect_poses();
    self.netcode.store_snapshot(tick, poses);
  }

  fn rewind_replay(&mut self, from: Tick) {
    let snap = self
      .netcode
      .snapshot_at(from)
      .cloned()
      .or_else(|| {
        // Fall back to nearest ring slot if exact tick missing
        self.netcode.ring.iter().find_map(|s| s.clone())
      });

    if snap.is_none() {
      return;
    }

    self.apply_poses(&snap.unwrap().poses);
    let auth = self.netcode.auth_tick.0;
    let mut checksums = HashMap::new();

    for t in from.0..auth {
      let tick = Tick(t);
      let intents = self.netcode.pending_for_tick(tick);
      self.apply_move_intents(&intents);
      self.integrate_physics(false);
      let poses = self.collect_poses();
      let cs = NetcodeState::checksum(&poses);
      checksums.insert(t, cs);
      if t % 2 == 0 {
        self.netcode.store_snapshot(tick, poses);
      }
    }

    let poses = self.collect_poses();
    self.netcode.push_correction(Correction {
      from_tick: from.0,
      checksums,
      poses,
    });
  }

  fn apply_move_intents(&mut self, intents: &[Intent]) {
    for intent in intents {
      if intent.verb == Verb::Move && intent.payload.len() >= 8 {
        let dx = f32::from_le_bytes(intent.payload[0..4].try_into().unwrap());
        let dy = f32::from_le_bytes(intent.payload[4..8].try_into().unwrap());
        if let Some(e) = self.find_avatar_for_fwau(intent.fwau) {
          if let Some(rec) = self.world.get_mut(e) {
            let speed = self.spec.ruleset.motion.max_speed;
            rec.velocity.linear.x = dx.clamp(-1.0, 1.0) * speed;
            rec.velocity.linear.y = dy.clamp(-1.0, 1.0) * speed;
            rec.dirty = true;
          }
        }
      } else if intent.verb == Verb::Blink && self.spec.ruleset.motion.blink {
        if intent.payload.len() >= 8 {
          let tx = f32::from_le_bytes(intent.payload[0..4].try_into().unwrap());
          let ty = f32::from_le_bytes(intent.payload[4..8].try_into().unwrap());
          if let Some(e) = self.find_avatar_for_fwau(intent.fwau) {
            if let Some(rec) = self.world.get_mut(e) {
              rec.transform.position.x = tx;
              rec.transform.position.y = ty;
              rec.velocity.linear = Vec3::ZERO;
              rec.dirty = true;
            }
          }
        }
      }
    }
  }

  fn integrate_physics(&mut self, advance_beams: bool) {
    let dt = self.spec.ruleset.dt_ms as f32 / 1000.0;
    let max_step = self.spec.ruleset.motion.max_speed * dt * 1.15;
    let entities: Vec<Entity> = self.world.awake_entities();

    for entity in entities {
      if let Some(rec) = self.world.get_mut(entity) {
        if rec.avatar.as_ref().map(|a| a.dead).unwrap_or(false) {
          continue;
        }

        if rec.brain.is_some() {
          let seed = rec.brain.as_ref().map(|b| b.seed).unwrap_or(0);
          let t = self.clock.tick.0 as f32;
          rec.velocity.linear.x = ((seed as f32 + t) * 0.1).sin() * 2.0;
          rec.velocity.linear.y = ((seed as f32 + t) * 0.13).cos() * 2.0;
        }

        let old = rec.transform.position;
        rec.transform.position.x += rec.velocity.linear.x * dt;
        rec.transform.position.y += rec.velocity.linear.y * dt;

        if self.spec.ruleset.motion.gravity > 0.0 {
          rec.velocity.linear.z -= self.spec.ruleset.motion.gravity * dt;
          rec.transform.position.z += rec.velocity.linear.z * dt;
          if rec.transform.position.z < 0.0 {
            rec.transform.position.z = 0.0;
            rec.velocity.linear.z = 0.0;
          }
        }

        // Speed hack detection for FWAU avatars
        if let Some(binding) = &rec.fwau_binding {
          let dx = rec.transform.position.x - old.x;
          let dy = rec.transform.position.y - old.y;
          let dist = (dx * dx + dy * dy).sqrt();
          if dist > max_step {
            rec.transform.position = old;
            rec.velocity.linear = Vec3::ZERO;
            self.netcode.record_reject(binding.fwau_id, IntentReject::SpeedHack);
          } else {
            self.player_last_pos.insert(binding.fwau_id, rec.transform.position);
          }
        }

        rec.dirty = true;
        self.grid.on_move(entity, rec.transform.position, rec.interest_lod);
      }
    }

    if advance_beams {
      // Beam advance handled by IslandManager in step_once
    }
  }

  pub fn run_ticks(&mut self, steps: u32, ledger: &mut EntropyLedger) -> Vec<ReplicationPacket> {
    let mut packets = Vec::new();
    for _ in 0..steps {
      self.step_once(ledger);
      packets.push(self.build_replication());
    }
    packets
  }

  pub fn step_once(&mut self, ledger: &mut EntropyLedger) {
    let tick = self.now();
    self.guardrails.begin_tick(self.spec.ruleset.dt_ms);

    // Gather intents scheduled for this tick from netcode pending buffer
    let mut all_intents = self.netcode.pending_for_tick(tick);
    for queue in self.intent_queues.values_mut() {
      all_intents.extend(queue.drain_for_tick(tick));
    }

    self.apply_move_intents(&all_intents);
    self.integrate_physics(false);
    self.update_sleep_states();
    self.step_islands_and_observe();
    self.regen_stamina();

    ledger.flush_due(tick);
    self.check_shard_boundaries();
    self.store_tick_snapshot(tick);
    self.netcode.clear_pending_through(tick);
  }

  fn step_islands_and_observe(&mut self) {
    let awake: HashSet<Entity> = self
      .world
      .all_entities()
      .into_iter()
      .filter(|e| self.world.get(*e).map(|r| r.sleep.awake).unwrap_or(false))
      .collect();

    for entity in &awake {
      if !self.was_awake.contains(entity) {
        self.island_mgr.queue_observe_entity(*entity);
      }
    }
    self.was_awake = awake.clone();

    let positions: Vec<(Entity, Vec3)> = self
      .world
      .all_entities()
      .iter()
      .filter_map(|&e| {
        self
          .world
          .get(e)
          .filter(|r| r.sleep.awake)
          .map(|r| (e, r.transform.position))
      })
      .collect();

    self.island_mgr.rebuild_from_positions(&positions, &awake);
    let budget = self.guardrails.config.step_budget_per_tick;
    if self.island_mgr.advance_with_budget(None, budget, true) {
      self.guardrails.record_budget_overrun();
      self.guardrails.record_throttled_tick();
    }
    self.island_mgr.collapse_observations();
  }

  fn check_shard_boundaries(&mut self) {
    if self.shard_bounds.is_none() {
      return;
    }
    let bounds = self.shard_bounds.as_ref().unwrap();
    for fwau in self.intent_queues.keys().copied().collect::<Vec<_>>() {
      if let Some(e) = self.find_avatar_for_fwau(fwau) {
        if let Some(rec) = self.world.get(e) {
          let pos = rec.transform.position;
          let target = if pos.x > bounds.max_x - bounds.overlap_m / 2.0 && bounds.id == 0 {
            1
          } else if pos.x < bounds.min_x + bounds.overlap_m / 2.0 && bounds.id == 1 {
            0
          } else {
            bounds.id
          };
          if target != bounds.id {
            self.pending_shard_handoffs.push((fwau, target));
          }
        }
      }
    }
  }

  fn update_sleep_states(&mut self) {
    let tick = self.clock.tick.0;
    for entity in self.world.all_entities() {
      let observed = self.is_entity_observed(entity);
      if let Some(rec) = self.world.get_mut(entity) {
        if observed {
          if !rec.sleep.awake {
            rec.sleep.awake = true;
            rec.dirty = true;
          }
        } else if rec.sleep.awake {
          if tick.saturating_sub(rec.sleep.slept_at.0) > self.sleep_delay_ticks {
            rec.sleep.awake = false;
            rec.sleep.snapshot_pos = rec.transform.position;
            rec.sleep.slept_at = Tick(tick);
            rec.dirty = true;
          }
        }
      }
    }
  }

  fn is_entity_observed(&self, entity: Entity) -> bool {
    for fwau in self.intent_queues.keys() {
      let interest = self.grid.interest_for(*fwau);
      if interest.contains(&entity) {
        return true;
      }
    }
    false
  }

  pub fn find_avatar_for_fwau(&self, fwau: FwauId) -> Option<Entity> {
    for entity in self.world.all_entities() {
      if let Some(rec) = self.world.get(entity) {
        if let Some(binding) = &rec.fwau_binding {
          if binding.fwau_id == fwau {
            return Some(entity);
          }
        }
      }
    }
    None
  }

  pub fn iuoc_for_fwau(&self, fwau: FwauId) -> Option<IuocId> {
    self
      .find_avatar_for_fwau(fwau)
      .and_then(|e| self.world.get(e))
      .and_then(|r| r.fwau_binding.as_ref())
      .map(|b| b.iuoc_id)
  }

  pub fn build_snapshot(&mut self) -> FrameSnapshot {
    let entities: Vec<EntitySnapshot> = self
      .world
      .all_entities()
      .iter()
      .filter_map(|&e| {
        self.world.get(e).map(|rec| EntitySnapshot {
          entity: rec.entity,
          name: rec.name.clone(),
          x: rec.transform.position.x,
          y: rec.transform.position.y,
          z: rec.transform.position.z,
          hp: rec.avatar.as_ref().map(|a| a.hp),
          stamina: rec.avatar.as_ref().map(|a| a.stamina),
          inventory: if rec.fwau_binding.is_some() {
            rec.avatar.as_ref().map(|a| a.inventory.clone())
          } else {
            None
          },
          is_player: rec.fwau_binding.is_some(),
          is_ai: rec.brain.is_some(),
          awake: rec.sleep.awake,
        })
      })
      .collect();

    let fwau_count = self.intent_queues.len();
    let ai_count = entities.iter().filter(|e| e.is_ai).count();

    FrameSnapshot {
      frame_id: self.spec.id.0,
      frame_name: self.spec.name.clone(),
      ruleset_id: self.spec.ruleset.id.clone(),
      tick: self.clock.tick.0,
      entities,
      fwau_count,
      ai_count,
      corrections: self.netcode.drain_corrections(),
      rejects: self
        .netcode
        .drain_rejects()
        .into_iter()
        .map(|(f, r)| RejectNotice {
          fwau: f.0,
          reason: format!("{:?}", r),
        })
        .collect(),
      island_profiler: Some(self.island_mgr.profiler.clone()),
      shard_id: self.shard_bounds.as_ref().map(|s| s.id),
      guardrails: Some(
        self
          .guardrails
          .report(self.island_mgr.profiler.within_budget),
      ),
      props: self.prop_store.all(),
    }
  }

  pub fn build_replication(&self) -> ReplicationPacket {
    let entities: Vec<EntitySnapshot> = self
      .world
      .all_entities()
      .iter()
      .filter_map(|&e| {
        self.world.get(e).filter(|r| r.dirty || r.sleep.awake).map(|rec| {
          EntitySnapshot {
            entity: rec.entity,
            name: rec.name.clone(),
            x: rec.transform.position.x,
            y: rec.transform.position.y,
            z: rec.transform.position.z,
            hp: rec.avatar.as_ref().map(|a| a.hp),
            stamina: rec.avatar.as_ref().map(|a| a.stamina),
            inventory: if rec.fwau_binding.is_some() {
              rec.avatar.as_ref().map(|a| a.inventory.clone())
            } else {
              None
            },
            is_player: rec.fwau_binding.is_some(),
            is_ai: rec.brain.is_some(),
            awake: rec.sleep.awake,
          }
        })
      })
      .collect();

    ReplicationPacket {
      tick: self.clock.tick.0,
      entities,
      band_changes: vec![],
    }
  }

  pub fn observe_island(&mut self, island: IslandId) -> Option<ObservationResult> {
    self.island_mgr.queue_observe(island);
    self
      .island_mgr
      .collapse_observations()
      .into_iter()
      .find(|(id, _)| *id == island)
      .map(|(_, r)| r)
  }

  pub fn psi_future_self(&self, fwau: FwauId) -> Vec<(String, f32)> {
    let entity = self.find_avatar_for_fwau(fwau);
    if entity.is_none() {
      return vec![];
    }
    let island_id = self
      .island_mgr
      .island_for_entity(entity.unwrap())
      .unwrap_or(IslandId(1));
    self.island_mgr.surface_odds(island_id)
  }

  pub fn drain_elapsed(&mut self, elapsed: Duration) -> u32 {
    let steps = self.clock.drain(elapsed);
    if steps == self.clock.max_catchup && self.clock.acc >= self.clock.dt {
      self.guardrails.record_stall();
    }
    steps
  }
}
