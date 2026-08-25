use serde::{Deserialize, Serialize};

/// Runtime topology for M11 multi-node PMR.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardNodeConfig {
    pub shard_id: Option<u32>,
    pub distributed: bool,
}

impl ShardNodeConfig {
    pub fn cluster() -> Self {
        Self {
            shard_id: None,
            distributed: false,
        }
    }

    pub fn single(shard_id: u32) -> Self {
        Self {
            shard_id: Some(shard_id),
            distributed: true,
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("TBC_SHARD_ID") {
            Ok(s) => Self::single(s.parse().unwrap_or(0)),
            Err(_) => Self::cluster(),
        }
    }

    pub fn label(&self) -> &'static str {
        if self.distributed {
            "distributed-shard"
        } else {
            "cluster"
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardNodeStatus {
    pub mode: String,
    pub shard_id: Option<u32>,
    pub distributed: bool,
    pub frame_count: usize,
}

/// Cross-shard spatial handoff published on `rww.shard.cross` (M11).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardCrossEvent {
    pub id: String,
    pub iuoc: u128,
    pub fwau: u128,
    pub from_shard: u32,
    pub to_shard: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tick: u64,
    pub ruleset: String,
}

impl ShardCrossEvent {
    pub fn new(
        iuoc: u128,
        fwau: u128,
        from_shard: u32,
        to_shard: u32,
        position: crate::types::Vec3,
        tick: u64,
        ruleset: &str,
    ) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            iuoc,
            fwau,
            from_shard,
            to_shard,
            x: position.x,
            y: position.y,
            z: position.z,
            tick,
            ruleset: ruleset.to_string(),
        }
    }
}

use crate::types::Vec3;

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

    pub fn for_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::shard_00()),
            1 => Some(Self::shard_01()),
            _ => None,
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
