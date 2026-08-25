use crate::types::{Entity, FwauId, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const CELL_SIZES: [f32; 3] = [32.0, 128.0, 512.0];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct CellKey {
    pub level: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Cell {
    pub entities: Vec<Entity>,
    pub subscribers: Vec<FwauId>,
}

pub fn cell_of(p: Vec3, level: u8) -> CellKey {
    let s = CELL_SIZES[level as usize];
    CellKey {
        level,
        x: (p.x / s).floor() as i32,
        y: (p.y / s).floor() as i32,
        z: (p.z / s).floor() as i32,
    }
}

pub fn lod_for_distance(distance: f32) -> u8 {
    if distance <= 32.0 {
        0
    } else if distance <= 128.0 {
        1
    } else {
        2
    }
}

/// Three-level hierarchical spatial hash for interest management.
pub struct HierGrid {
    pub map: HashMap<CellKey, Cell>,
    entity_positions: HashMap<Entity, Vec3>,
    entity_cells: HashMap<Entity, CellKey>,
    fwau_positions: HashMap<FwauId, Vec3>,
}

impl HierGrid {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            entity_positions: HashMap::new(),
            entity_cells: HashMap::new(),
            fwau_positions: HashMap::new(),
        }
    }

    fn cell_mut(&mut self, key: CellKey) -> &mut Cell {
        self.map.entry(key).or_default()
    }

    pub fn insert_entity(&mut self, entity: Entity, pos: Vec3, lod: u8) {
        let key = cell_of(pos, lod);
        self.entity_positions.insert(entity, pos);
        self.entity_cells.insert(entity, key);
        self.cell_mut(key).entities.push(entity);
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(key) = self.entity_cells.remove(&entity) {
            if let Some(cell) = self.map.get_mut(&key) {
                cell.entities.retain(|&e| e != entity);
            }
        }
        self.entity_positions.remove(&entity);
    }

    pub fn on_move(&mut self, entity: Entity, new_pos: Vec3, lod: u8) -> Vec<FwauId> {
        let old_key = self.entity_cells.get(&entity).copied();
        let new_key = cell_of(new_pos, lod);
        self.entity_positions.insert(entity, new_pos);

        if old_key == Some(new_key) {
            return vec![];
        }

        if let Some(old) = old_key {
            if let Some(cell) = self.map.get_mut(&old) {
                cell.entities.retain(|&e| e != entity);
            }
        }
        self.entity_cells.insert(entity, new_key);
        self.cell_mut(new_key).entities.push(entity);

        let old_subs: HashSet<FwauId> = old_key
            .and_then(|k| self.map.get(&k))
            .map(|c| c.subscribers.iter().copied().collect())
            .unwrap_or_default();
        let new_subs: HashSet<FwauId> = self
            .map
            .get(&new_key)
            .map(|c| c.subscribers.iter().copied().collect())
            .unwrap_or_default();

        old_subs.symmetric_difference(&new_subs).copied().collect()
    }

    pub fn subscribe_fwau(&mut self, fwau: FwauId, pos: Vec3, lod: u8) {
        self.fwau_positions.insert(fwau, pos);
        let key = cell_of(pos, lod);
        let neighborhood = neighborhood_keys(key);
        for nk in neighborhood {
            self.cell_mut(nk).subscribers.push(fwau);
        }
    }

    pub fn update_fwau_pos(&mut self, fwau: FwauId, pos: Vec3, lod: u8) {
        self.fwau_positions.insert(fwau, pos);
        // Simplified: re-subscribe on move (M3 debug path)
        self.subscribe_fwau(fwau, pos, lod);
    }

    pub fn query(&self, pos: Vec3, radius: f32) -> Vec<Entity> {
        let level = lod_for_distance(radius);
        let center = cell_of(pos, level);
        let mut result = Vec::new();
        for nk in neighborhood_keys(center) {
            if let Some(cell) = self.map.get(&nk) {
                for &e in &cell.entities {
                    let ep = self.entity_positions.get(&e).copied().unwrap_or(Vec3::ZERO);
                    if ep.distance(&pos) <= radius {
                        result.push(e);
                    }
                }
            }
        }
        result
    }

    pub fn interest_for(&self, fwau: FwauId) -> Vec<Entity> {
        let pos = self
            .fwau_positions
            .get(&fwau)
            .copied()
            .unwrap_or(Vec3::ZERO);
        self.query(pos, 128.0)
    }

    pub fn entity_position(&self, entity: Entity) -> Option<Vec3> {
        self.entity_positions.get(&entity).copied()
    }
}

fn neighborhood_keys(center: CellKey) -> Vec<CellKey> {
    let mut keys = Vec::with_capacity(27);
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                keys.push(CellKey {
                    level: center.level,
                    x: center.x + dx,
                    y: center.y + dy,
                    z: center.z + dz,
                });
            }
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_move_stays_o1_when_same_cell() {
        let mut grid = HierGrid::new();
        let e = Entity {
            index: 0,
            generation: 0,
        };
        grid.insert_entity(e, Vec3::new(10.0, 10.0, 0.0), 1);
        let dirty = grid.on_move(e, Vec3::new(15.0, 15.0, 0.0), 1);
        assert!(dirty.is_empty());
    }

    #[test]
    fn query_returns_nearby_entities() {
        let mut grid = HierGrid::new();
        let e1 = Entity {
            index: 1,
            generation: 0,
        };
        let e2 = Entity {
            index: 2,
            generation: 0,
        };
        grid.insert_entity(e1, Vec3::new(0.0, 0.0, 0.0), 1);
        grid.insert_entity(e2, Vec3::new(500.0, 0.0, 0.0), 1);
        let found = grid.query(Vec3::ZERO, 64.0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], e1);
    }
}
