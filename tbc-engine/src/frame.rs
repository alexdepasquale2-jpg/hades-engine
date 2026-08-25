use crate::beam::{BranchEntity, IslandState, ProbabilitySurface, ObservationResult};
use crate::clock::DeltaTClock;
use crate::ecs::World;
use crate::grid::HierGrid;
use crate::intent::{Intent, IntentQueue, Verb};
use crate::iuoc::{EntityRef, IuocRegistry};
use crate::ledger::{EntropyLedger, ResolvedAction};
use crate::ruleset::Ruleset;
use crate::types::{
  Entity, FwauId, FrameId, IslandId, IuocId, Tick, Vec3,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
  pub is_player: bool,
  pub is_ai: bool,
  pub awake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameSnapshot {
  pub frame_id: u32,
  pub tick: u64,
  pub entities: Vec<EntitySnapshot>,
  pub fwau_count: usize,
  pub ai_count: usize,
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
  pub islands: HashMap<IslandId, ProbabilitySurface>,
  pub intent_queues: HashMap<FwauId, IntentQueue>,
  pub rng_seed: u64,
  pub stall_count: u32,
  sleep_delay_ticks: u64,
}

impl Frame {
  pub fn new(spec: FrameSpec) -> Self {
    let dt_ms = spec.ruleset.dt_ms as u64;
    let sleep_delay_ticks = (spec.ruleset.sleep.delay_s * 1000.0 / dt_ms as f32) as u64;
    Self {
      clock: DeltaTClock::new(dt_ms),
      spec,
      world: World::new(),
      grid: HierGrid::new(),
      islands: HashMap::new(),
      intent_queues: HashMap::new(),
      rng_seed: 0xA00,
      stall_count: 0,
      sleep_delay_ticks,
    }
  }

  pub fn now(&self) -> Tick {
    self.clock.now()
  }

  pub fn spawn_demo_world(&mut self, ai_count: usize) {
    for i in 0..ai_count {
      let angle = (i as f32) * 0.5;
      let pos = Vec3::new(
        (angle.cos() * 50.0) + (i as f32 * 3.0),
        angle.sin() * 50.0,
        0.0,
      );
      let e = self.world.spawn_ai_guy(&format!("AI-Guy-{}", i), pos, i as u64 + 1);
      self.grid.insert_entity(e, pos, 1);
      self.ensure_island_for_entity(e);
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

    self.ensure_island_for_entity(entity);
    Ok(fwau_id)
  }

  fn ensure_island_for_entity(&mut self, entity: Entity) {
    let island_id = IslandId(entity.index as u64 + 1);
    if !self.islands.contains_key(&island_id) {
      let present = IslandState {
        entities: vec![BranchEntity {
          id: entity.index,
          x: 0.0,
          y: 0.0,
          vx: 1.0,
          vy: 0.0,
        }],
        tick: self.clock.tick.0,
      };
      let surface = ProbabilitySurface::new(island_id, present, self.rng_seed + island_id.0);
      self.islands.insert(island_id, surface);
    }
  }

  pub fn submit_intent(&mut self, intent: Intent) -> bool {
    if let Some(queue) = self.intent_queues.get_mut(&intent.fwau) {
      queue.push(intent)
    } else {
      false
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

    // (1) gather intents
    let mut all_intents: Vec<Intent> = Vec::new();
    for queue in self.intent_queues.values_mut() {
      all_intents.extend(queue.drain_for_tick(tick));
    }

    // (2) movement from intents
    for intent in &all_intents {
      if intent.verb == Verb::Move && intent.payload.len() >= 8 {
        let dx = f32::from_le_bytes(intent.payload[0..4].try_into().unwrap());
        let dy = f32::from_le_bytes(intent.payload[4..8].try_into().unwrap());
        let entity = self.find_avatar_for_fwau(intent.fwau);
        if entity.is_some() {
          let e = entity.unwrap();
          if let Some(rec) = self.world.get_mut(e) {
            let speed = self.spec.ruleset.motion.max_speed;
            rec.velocity.linear.x = dx.clamp(-1.0, 1.0) * speed;
            rec.velocity.linear.y = dy.clamp(-1.0, 1.0) * speed;
            rec.dirty = true;
          }
        }
      }
    }

    // (3) integrate movement + AI
    let dt = self.spec.ruleset.dt_ms as f32 / 1000.0;
    let entities: Vec<Entity> = self.world.awake_entities();
    for entity in entities {
      if let Some(rec) = self.world.get_mut(entity) {
        if rec.avatar.as_ref().map(|a| a.dead).unwrap_or(false) {
          continue;
        }

        if rec.brain.is_some() {
          // Simple utility AI wander
          let seed = rec.brain.as_ref().map(|b| b.seed).unwrap_or(0);
          let t = self.clock.tick.0 as f32;
          rec.velocity.linear.x = ((seed as f32 + t) * 0.1).sin() * 2.0;
          rec.velocity.linear.y = ((seed as f32 + t) * 0.13).cos() * 2.0;
        }

        rec.transform.position.x += rec.velocity.linear.x * dt;
        rec.transform.position.y += rec.velocity.linear.y * dt;

        // gravity
        if self.spec.ruleset.motion.gravity > 0.0 {
          rec.velocity.linear.z -= self.spec.ruleset.motion.gravity * dt;
          rec.transform.position.z += rec.velocity.linear.z * dt;
          if rec.transform.position.z < 0.0 {
            rec.transform.position.z = 0.0;
            rec.velocity.linear.z = 0.0;
          }
        }

        rec.dirty = true;
        self.grid.on_move(entity, rec.transform.position, rec.interest_lod);
      }
    }

    // (4) advance probability surfaces
    for surface in self.islands.values_mut() {
      surface.advance(None);
    }

    // (5) sleep/wake
    self.update_sleep_states();

    // (6) ledger enqueue for attacks
    for intent in &all_intents {
      if intent.verb == Verb::Attack {
        if let Some(fwau) = self.fwau_from_intent(intent.fwau) {
          let action = ResolvedAction {
            id: intent.seq as u128,
            aid: 0.0,
            harm: 0.5,
            ego: 0.0,
            coerce: 0.0,
            perf: 0.0,
          };
          let due = Tick(tick.0 + 600); // ~30s at 20Hz for demo
          ledger.enqueue_consequence(fwau, &action, due);
        }
      }
    }

    // (7) flush ledger events that are due (demo: immediate short delay)
    ledger.flush_due(tick);
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

  fn find_avatar_for_fwau(&self, fwau: FwauId) -> Option<Entity> {
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

  fn fwau_from_intent(&self, fwau: FwauId) -> Option<IuocId> {
    self
      .find_avatar_for_fwau(fwau)
      .and_then(|e| self.world.get(e))
      .and_then(|r| r.fwau_binding.as_ref())
      .map(|b| b.iuoc_id)
  }

  pub fn build_snapshot(&self) -> FrameSnapshot {
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
      tick: self.clock.tick.0,
      entities,
      fwau_count,
      ai_count,
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
    self.islands.get_mut(&island).map(|s| s.observe())
  }

  pub fn psi_future_self(&self, fwau: FwauId) -> Vec<(String, f32)> {
    let entity = self.find_avatar_for_fwau(fwau);
    if entity.is_none() {
      return vec![];
    }
    let island_id = IslandId(entity.unwrap().index as u64 + 1);
    if let Some(surface) = self.islands.get(&island_id) {
      let total: f32 = surface.beam.iter().map(|b| b.weight).sum();
      surface
        .beam
        .iter()
        .take(3)
        .map(|b| {
          let p = if total > 0.0 {
            b.weight / total
          } else {
            0.0
          };
          (format!("branch:{:?}", b.path), p)
        })
        .collect()
    } else {
      vec![]
    }
  }

  pub fn drain_elapsed(&mut self, elapsed: Duration) -> u32 {
    let steps = self.clock.drain(elapsed);
    if steps == self.clock.max_catchup && self.clock.acc >= self.clock.dt {
      self.stall_count += 1;
    }
    steps
  }
}
