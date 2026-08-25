use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct IuocId(pub u128);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct FwauId(pub u128);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct AvatarId(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct FrameId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct IslandId(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
pub struct PacketId(pub u128);

#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default, PartialOrd, Ord,
)]
pub struct Tick(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl Entity {
    pub const INVALID: Entity = Entity {
        index: u32::MAX,
        generation: u32::MAX,
    };

    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::INVALID
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}:{}", self.index, self.generation)
    }
}

/// Quality scalar S ∈ [0, 1]. Lower is better (lower entropy).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct QualityScalar(pub f32);

impl QualityScalar {
    pub const INITIAL: QualityScalar = QualityScalar(0.50);

    pub fn clamped(v: f32) -> Self {
        QualityScalar(v.clamp(0.0, 1.0))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn band(&self) -> QualityBand {
        QualityBand::from_scalar(self.0)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityBand {
    Turbulent,
    Restless,
    Settled,
    Coherent,
    Quiet,
}

impl QualityBand {
    pub fn from_scalar(s: f32) -> Self {
        if s >= 0.72 {
            QualityBand::Turbulent
        } else if s >= 0.58 {
            QualityBand::Restless
        } else if s >= 0.42 {
            QualityBand::Settled
        } else if s >= 0.28 {
            QualityBand::Coherent
        } else {
            QualityBand::Quiet
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            QualityBand::Turbulent => "Turbulent",
            QualityBand::Restless => "Restless",
            QualityBand::Settled => "Settled",
            QualityBand::Coherent => "Coherent",
            QualityBand::Quiet => "Quiet",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
