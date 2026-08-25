use crate::types::Vec3;
use serde::{Deserialize, Serialize};

/// Spatial partition for seamless PMR sharding (spec §10).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardBounds {
  pub id: u32,
  pub name: String,
  pub min_x: f32,
  pub max_x: f32,
  pub overlap_m: f32,
}

impl ShardBounds {
  pub fn shard_00() -> Self {
    Self {
      id: 0,
      name: "shard-00".into(),
      min_x: -500.0,
      max_x: 50.0,
      overlap_m: 64.0,
    }
  }

  pub fn shard_01() -> Self {
    Self {
      id: 1,
      name: "shard-01".into(),
      min_x: -50.0,
      max_x: 500.0,
      overlap_m: 64.0,
    }
  }

  pub fn owns(&self, p: Vec3) -> bool {
    p.x >= self.min_x && p.x <= self.max_x
  }

  pub fn in_overlap_strip(&self, p: Vec3) -> bool {
    if self.id == 0 {
      p.x >= self.max_x - self.overlap_m
    } else {
      p.x <= self.min_x + self.overlap_m
    }
  }

  pub fn home_shard_for(&self, p: Vec3) -> u32 {
    if p.x < 0.0 {
      0
    } else {
      1
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardHandoff {
  pub fwau: u128,
  pub from_shard: u32,
  pub to_shard: u32,
  pub position: Vec3,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn overlap_strip_detected() {
    let s0 = ShardBounds::shard_00();
    assert!(s0.in_overlap_strip(Vec3::new(40.0, 0.0, 0.0)));
    assert!(!s0.in_overlap_strip(Vec3::new(-200.0, 0.0, 0.0)));
  }
}
