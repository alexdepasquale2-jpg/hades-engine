use crate::types::IslandId;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde::{Deserialize, Serialize};

pub const BEAM_WIDTH: usize = 16;
pub const ACTION_FANOUT: usize = 6;
pub const DEPTH: usize = 8;
pub const PRUNE_FLOOR: f32 = 1e-4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IslandState {
    pub entities: Vec<BranchEntity>,
    pub tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchEntity {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub state: IslandState,
    pub weight: f32,
    pub path: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservationResult {
    pub resolved: IslandState,
    pub odds: Vec<(String, f32)>,
}

/// Probable reality surface — beam of weighted branches per causality island.
pub struct ProbabilitySurface {
    pub island: IslandId,
    pub beam: Vec<Branch>,
    pub depth: u8,
    pub rng: Pcg64,
    pub steps_per_tick: usize,
}

impl ProbabilitySurface {
    pub fn new(island: IslandId, present: IslandState, seed: u64) -> Self {
        let branch = Branch {
            state: present,
            weight: 1.0,
            path: vec![],
        };
        Self {
            island,
            beam: vec![branch],
            depth: DEPTH as u8,
            rng: Pcg64::seed_from_u64(seed),
            steps_per_tick: 0,
        }
    }

    pub fn advance(&mut self, intent_action: Option<u8>) {
        self.advance_with_depth(intent_action, self.depth);
    }

    pub fn advance_with_depth(&mut self, intent_action: Option<u8>, max_depth: u8) {
        let mut candidates: Vec<Branch> = Vec::with_capacity(BEAM_WIDTH * ACTION_FANOUT);
        self.steps_per_tick = 0;
        let depth = max_depth.min(self.depth);

        for d in 1..=depth {
            candidates.clear();
            for branch in &self.beam {
                for a in 0..ACTION_FANOUT {
                    let action = a as u8;
                    let mut new_state = branch.state.clone();
                    step_island(&mut new_state, action);
                    let w = branch.weight * ruleset_prob(action) * intent_bias(intent_action, action, d);

                    if w < PRUNE_FLOOR {
                        continue;
                    }

                    let mut path = branch.path.clone();
                    path.push(action);
                    candidates.push(Branch {
                        state: new_state,
                        weight: w,
                        path,
                    });
                    self.steps_per_tick += 1;
                }
            }
            candidates.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
            candidates.truncate(BEAM_WIDTH);
            std::mem::swap(&mut self.beam, &mut candidates);
        }
    }

    pub fn observe(&mut self) -> ObservationResult {
        let total: f32 = self.beam.iter().map(|b| b.weight).sum();
        let odds: Vec<(String, f32)> = self
            .beam
            .iter()
            .take(3)
            .map(|b| {
                let label = format!("path:{:?}", b.path);
                let p = if total > 0.0 { b.weight / total } else { 0.0 };
                (label, p)
            })
            .collect();

        let r = self.rng.gen::<f32>() * total;
        let mut cumulative = 0.0f32;
        let mut resolved = self.beam[0].state.clone();
        for branch in &self.beam {
            cumulative += branch.weight;
            if r <= cumulative {
                resolved = branch.state.clone();
                break;
            }
        }

        ObservationResult { resolved, odds }
    }
}

fn step_island(state: &mut IslandState, action: u8) {
    state.tick += 1;
    for ent in &mut state.entities {
        match action % 6 {
            0 => ent.vx += 0.1,
            1 => ent.vx -= 0.1,
            2 => ent.vy += 0.1,
            3 => ent.vy -= 0.1,
            4 => ent.vx *= 0.95,
            5 => ent.vy *= 0.95,
            _ => {}
        }
        ent.x += ent.vx;
        ent.y += ent.vy;
    }
}

fn ruleset_prob(action: u8) -> f32 {
    0.15 + (action as f32 * 0.02)
}

fn intent_bias(intent: Option<u8>, action: u8, depth: u8) -> f32 {
    if intent == Some(action) && depth == 1 {
        1.0
    } else if intent.is_some() && intent != Some(action) && depth == 1 {
        1e-6
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_steps_match_spec() {
        let present = IslandState {
            entities: vec![BranchEntity {
                id: 0,
                x: 0.0,
                y: 0.0,
                vx: 1.0,
                vy: 0.0,
            }],
            tick: 0,
        };
        let mut surface = ProbabilitySurface::new(IslandId(1), present, 42);
        surface.advance(None);
        // D * B * A = 8 * 16 * 6 = 768 at each depth level... spec says per island per tick
        // Worst-case per spec: D × B × A = 768 step() calls per island per tick.
        assert!(surface.steps_per_tick >= ACTION_FANOUT);
        assert!(surface.steps_per_tick <= DEPTH * BEAM_WIDTH * ACTION_FANOUT);
    }
}
