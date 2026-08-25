use crate::beam::{IslandState, ObservationResult, ProbabilitySurface};
use crate::types::{Entity, IslandId, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const TARGET_ISLANDS: usize = 40;
pub const LOOKAHEAD_M: f32 = 120.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IslandProfiler {
  pub island_count: usize,
  pub total_steps_last_tick: usize,
  pub max_steps_per_island: usize,
  pub observations_last_tick: usize,
  pub ticks_profiled: u64,
  pub within_budget: bool,
}

impl Default for IslandProfiler {
  fn default() -> Self {
    Self {
      island_count: 0,
      total_steps_last_tick: 0,
      max_steps_per_island: 0,
      observations_last_tick: 0,
      ticks_profiled: 0,
      within_budget: true,
    }
  }
}

impl IslandProfiler {
  /// Spec budget: ~768 steps/island × 40 islands ≈ 30k steps/tick at 20 Hz.
  pub const STEP_BUDGET_PER_TICK: usize = 35_000;

  pub fn record_tick(&mut self, island_count: usize, steps: usize, max_steps: usize, obs: usize) {
    self.island_count = island_count;
    self.total_steps_last_tick = steps;
    self.max_steps_per_island = max_steps;
    self.observations_last_tick = obs;
    self.ticks_profiled += 1;
    self.within_budget = steps <= Self::STEP_BUDGET_PER_TICK;
  }
}

/// Manages causality islands, beam advance, observe-collapse, and profiling.
pub struct IslandManager {
  pub islands: HashMap<IslandId, ProbabilitySurface>,
  entity_to_island: HashMap<Entity, IslandId>,
  pub profiler: IslandProfiler,
  observe_queue: HashSet<IslandId>,
  rng_seed: u64,
  next_island_id: u64,
}

impl IslandManager {
  pub fn new(rng_seed: u64) -> Self {
    Self {
      islands: HashMap::new(),
      entity_to_island: HashMap::new(),
      profiler: IslandProfiler::default(),
      observe_queue: HashSet::new(),
      rng_seed,
      next_island_id: 1,
    }
  }

  pub fn rebuild_from_positions(
    &mut self,
    positions: &[(Entity, Vec3)],
    awake: &HashSet<Entity>,
  ) {
    self.entity_to_island.clear();
    self.islands.clear();
    self.next_island_id = 1;

    let mut clusters: Vec<Vec<(Entity, Vec3)>> = Vec::new();

    for &(entity, pos) in positions {
      if !awake.contains(&entity) {
        continue;
      }
      let mut merged = false;
      for cluster in &mut clusters {
        if let Some((_, center)) = cluster.first() {
          if pos.distance(center) <= LOOKAHEAD_M {
            cluster.push((entity, pos));
            merged = true;
            break;
          }
        }
      }
      if !merged {
        clusters.push(vec![(entity, pos)]);
      }
    }

  // Merge clusters until we have at most TARGET_ISLANDS (greedy merge smallest)
    while clusters.len() > TARGET_ISLANDS {
      clusters.sort_by_key(|c| c.len());
      let a = clusters.remove(0);
      let b = clusters.remove(0);
      let merged: Vec<(Entity, Vec3)> = a.into_iter().chain(b).collect();
      clusters.push(merged);
    }

    for cluster in clusters {
      let island_id = IslandId(self.next_island_id);
      self.next_island_id += 1;

      let entities: Vec<crate::beam::BranchEntity> = cluster
        .iter()
        .map(|(e, p)| crate::beam::BranchEntity {
          id: e.index,
          x: p.x,
          y: p.y,
          vx: 0.0,
          vy: 0.0,
        })
        .collect();

      let present = IslandState {
        entities,
        tick: 0,
      };
      let surface =
        ProbabilitySurface::new(island_id, present, self.rng_seed + island_id.0);
      self.islands.insert(island_id, surface);

      for (entity, _) in cluster {
        self.entity_to_island.insert(entity, island_id);
      }
    }
  }

  pub fn queue_observe(&mut self, island: IslandId) {
    self.observe_queue.insert(island);
  }

  pub fn queue_observe_entity(&mut self, entity: Entity) {
    if let Some(id) = self.entity_to_island.get(&entity) {
      self.observe_queue.insert(*id);
    }
  }

  pub fn advance_all(&mut self, intent_action: Option<u8>) {
    let mut total_steps = 0;
    let mut max_steps = 0;
    for surface in self.islands.values_mut() {
      surface.advance(intent_action);
      total_steps += surface.steps_per_tick;
      max_steps = max_steps.max(surface.steps_per_tick);
    }
    let obs_count = self.observe_queue.len();
    self.profiler.record_tick(self.islands.len(), total_steps, max_steps, obs_count);
  }

  pub fn collapse_observations(&mut self) -> Vec<(IslandId, ObservationResult)> {
    let mut results = Vec::new();
    for island_id in self.observe_queue.drain() {
      if let Some(surface) = self.islands.get_mut(&island_id) {
        let result = surface.observe();
        results.push((island_id, result));
      }
    }
    self.profiler.observations_last_tick = results.len();
    results
  }

  pub fn island_for_entity(&self, entity: Entity) -> Option<IslandId> {
    self.entity_to_island.get(&entity).copied()
  }

  pub fn surface_odds(&self, island: IslandId) -> Vec<(String, f32)> {
    let surface = match self.islands.get(&island) {
      Some(s) => s,
      None => return vec![],
    };
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
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::Entity;

  #[test]
  fn clusters_nearby_entities() {
    let mut mgr = IslandManager::new(42);
    let e1 = Entity {
      index: 1,
      generation: 0,
    };
    let e2 = Entity {
      index: 2,
      generation: 0,
    };
    let awake = HashSet::from([e1, e2]);
    let positions = vec![
      (e1, Vec3::new(0.0, 0.0, 0.0)),
      (e2, Vec3::new(10.0, 0.0, 0.0)),
    ];
    mgr.rebuild_from_positions(&positions, &awake);
    assert!(mgr.islands.len() <= TARGET_ISLANDS);
    assert!(!mgr.islands.is_empty());
  }

  #[test]
  fn profiler_budget_at_40_islands() {
    let mut mgr = IslandManager::new(99);
    let mut positions = Vec::new();
    let mut awake = HashSet::new();
    for i in 0..80 {
      let e = Entity {
        index: i,
        generation: 0,
      };
      awake.insert(e);
      positions.push((e, Vec3::new(i as f32 * 3.0, 0.0, 0.0)));
    }
    mgr.rebuild_from_positions(&positions, &awake);
    mgr.advance_all(None);
    assert!(mgr.profiler.island_count <= TARGET_ISLANDS);
    assert!(mgr.profiler.within_budget);
  }
}
