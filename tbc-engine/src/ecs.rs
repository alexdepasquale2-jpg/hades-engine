use crate::types::{Entity, FwauId, IuocId, Tick, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
  pub position: Vec3,
  pub rotation: f32,
}

impl Default for Transform {
  fn default() -> Self {
    Self {
      position: Vec3::ZERO,
      rotation: 0.0,
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Velocity {
  pub linear: Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FwauBinding {
  pub fwau_id: FwauId,
  pub iuoc_id: IuocId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BrainKind {
  Utility,
  BehaviorTree,
  Ml,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brain {
  pub kind: BrainKind,
  pub seed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SleepState {
  pub snapshot_pos: Vec3,
  pub seed: u64,
  pub slept_at: Tick,
  pub awake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AvatarState {
  pub hp: f32,
  pub stamina: f32,
  pub dead: bool,
}

#[derive(Clone, Debug)]
pub struct EntityRecord {
  pub entity: Entity,
  pub transform: Transform,
  pub velocity: Velocity,
  pub fwau_binding: Option<FwauBinding>,
  pub brain: Option<Brain>,
  pub sleep: SleepState,
  pub avatar: Option<AvatarState>,
  pub interest_lod: u8,
  pub name: String,
  pub dirty: bool,
}

pub struct World {
  entities: Vec<Option<EntityRecord>>,
  free_list: Vec<u32>,
  generation: Vec<u32>,
}

impl World {
  pub fn new() -> Self {
    Self {
      entities: Vec::new(),
      free_list: Vec::new(),
      generation: Vec::new(),
    }
  }

  pub fn spawn(&mut self, name: &str) -> Entity {
    let index = if let Some(idx) = self.free_list.pop() {
      idx
    } else {
      let idx = self.entities.len() as u32;
      self.entities.push(None);
      self.generation.push(0);
      idx
    };
    let gen = self.generation[index as usize];
    let entity = Entity {
      index,
      generation: gen,
    };
    self.entities[index as usize] = Some(EntityRecord {
      entity,
      transform: Transform::default(),
      velocity: Velocity::default(),
      fwau_binding: None,
      brain: None,
      sleep: SleepState {
        awake: true,
        ..Default::default()
      },
      avatar: None,
      interest_lod: 1,
      name: name.to_string(),
      dirty: true,
    });
    entity
  }

  pub fn despawn(&mut self, entity: Entity) {
    if let Some(slot) = self.entities.get_mut(entity.index as usize) {
      if let Some(rec) = slot {
        if rec.entity.generation == entity.generation {
          *slot = None;
          self.generation[entity.index as usize] += 1;
          self.free_list.push(entity.index);
        }
      }
    }
  }

  pub fn get(&self, entity: Entity) -> Option<&EntityRecord> {
    self.entities
      .get(entity.index as usize)
      .and_then(|s| s.as_ref())
      .filter(|r| r.entity.generation == entity.generation)
  }

  pub fn get_mut(&mut self, entity: Entity) -> Option<&mut EntityRecord> {
    self.entities
      .get_mut(entity.index as usize)
      .and_then(|s| s.as_mut())
      .filter(|r| r.entity.generation == entity.generation)
  }

  pub fn all_entities(&self) -> Vec<Entity> {
    self
      .entities
      .iter()
      .filter_map(|s| s.as_ref().map(|r| r.entity))
      .collect()
  }

  pub fn awake_entities(&self) -> Vec<Entity> {
    self
      .entities
      .iter()
      .filter_map(|s| {
        s.as_ref().filter(|r| r.sleep.awake).map(|r| r.entity)
      })
      .collect()
  }

  pub fn spawn_ai_guy(&mut self, name: &str, pos: Vec3, seed: u64) -> Entity {
    let e = self.spawn(name);
    if let Some(rec) = self.get_mut(e) {
      rec.transform.position = pos;
      rec.brain = Some(Brain {
        kind: BrainKind::Utility,
        seed,
      });
      rec.sleep.awake = true;
    }
    e
  }

  pub fn bind_avatar(&mut self, entity: Entity, fwau: FwauId, iuoc: IuocId) {
    if let Some(rec) = self.get_mut(entity) {
      rec.fwau_binding = Some(FwauBinding {
        fwau_id: fwau,
        iuoc_id: iuoc,
      });
      rec.avatar = Some(AvatarState {
        hp: 100.0,
        stamina: 100.0,
        dead: false,
      });
      rec.sleep.awake = true;
      rec.dirty = true;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generational_despawn() {
    let mut world = World::new();
    let e = world.spawn("test");
    world.despawn(e);
    assert!(world.get(e).is_none());
  }
}
