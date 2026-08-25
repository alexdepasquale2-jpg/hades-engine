//! M13 — ruleset-driven combat, interact, and conservation hooks.

use crate::ecs::World;
use crate::frame::Frame;
use crate::ledger::{EntropyLedger, ResolvedAction};
use crate::ruleset::{AttackPolicy, InteractPolicy};
use crate::types::{Entity, FwauId, IuocId, Tick, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackResult {
    pub hit: bool,
    pub target_name: String,
    pub target_entity: Option<u32>,
    pub damage: f32,
    pub target_hp: f32,
    pub killed: bool,
    pub player_stamina: f32,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractResult {
    pub hit: bool,
    pub target_name: String,
    pub item_granted: Option<String>,
    pub inventory: Vec<String>,
    pub message: String,
}

pub fn distance2(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn nearest_interactable(
    world: &World,
    origin: Vec3,
    range: f32,
    exclude_player: Option<Entity>,
) -> Option<(Entity, String, Vec3)> {
    let mut best: Option<(Entity, String, Vec3, f32)> = None;
    for entity in world.all_entities() {
        if exclude_player == Some(entity) {
            continue;
        }
        let rec = world.get(entity)?;
        if !rec.sleep.awake {
            continue;
        }
        if rec.brain.is_none() && rec.fwau_binding.is_none() {
            continue;
        }
        let d = distance2(origin, rec.transform.position);
        if d > range {
            continue;
        }
        let better = match best {
            None => true,
            Some((_, _, _, bd)) => d < bd,
        };
        if better {
            best = Some((entity, rec.name.clone(), rec.transform.position, d));
        }
    }
    best.map(|(e, n, p, _)| (e, n, p))
}

impl Frame {
    pub fn try_attack(
        &mut self,
        fwau: FwauId,
        target: Option<Entity>,
        ledger: &mut EntropyLedger,
    ) -> AttackResult {
        let policy = &self.spec.ruleset.verbs.attack;
        if !policy.enabled {
            return AttackResult {
                hit: false,
                target_name: "".into(),
                target_entity: None,
                damage: 0.0,
                target_hp: 0.0,
                killed: false,
                player_stamina: 0.0,
                message: "Attack disabled by ruleset".into(),
            };
        }

        let player_entity = self
            .find_avatar_for_fwau(fwau)
            .ok_or_else(|| "no avatar".to_string());
        if player_entity.is_err() {
            return fail_attack("No avatar bound");
        }
        let player_entity = player_entity.unwrap();
        let player_pos = self
            .world
            .get(player_entity)
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);

        let (target_entity, target_name) = if let Some(t) = target {
            let name = self
                .world
                .get(t)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            (t, name)
        } else {
            match nearest_interactable(&self.world, player_pos, policy.range_m, Some(player_entity))
            {
                Some((e, name, _)) => (e, name),
                None => {
                    return fail_attack("No target in range");
                }
            }
        };

        let target_pos = self
            .world
            .get(target_entity)
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);
        if distance2(player_pos, target_pos) > policy.range_m {
            return fail_attack("Target out of range");
        }

        let mut player_stamina = 0.0;
        if let Some(rec) = self.world.get_mut(player_entity) {
            let avatar = rec
                .avatar
                .as_mut()
                .ok_or_else(|| "no avatar state".to_string());
            if avatar.is_err() {
                return fail_attack("No avatar state");
            }
            let avatar = avatar.unwrap();
            if avatar.dead {
                return fail_attack("You are unbound");
            }
            if avatar.stamina < policy.stamina_cost {
                return fail_attack("Stamina exhausted");
            }
            avatar.stamina -= policy.stamina_cost;
            player_stamina = avatar.stamina;
            rec.dirty = true;
        }

        let mut target_hp;
        let mut killed = false;
        let now = self.now().0;
        let attacker_iuoc = self.iuoc_for_fwau(fwau);

        if let Some(rec) = self.world.get_mut(target_entity) {
            if let Some(avatar) = &mut rec.avatar {
                avatar.hp -= policy.damage;
                target_hp = avatar.hp;
                if avatar.hp <= 0.0 {
                    avatar.hp = 0.0;
                    avatar.dead = true;
                    killed = true;
                    rec.velocity.linear = Vec3::ZERO;
                }
                rec.dirty = true;
            } else {
                return fail_attack("Target cannot be harmed");
            }
        } else {
            return fail_attack("Target missing");
        }

        if let Some(iuoc) = attacker_iuoc {
            ledger.enqueue_consequence(
                iuoc,
                &ResolvedAction {
                    id: now as u128,
                    aid: 0.0,
                    harm: policy.harm_entropy,
                    ego: 0.1,
                    coerce: 0.0,
                    perf: 0.0,
                },
                Tick(now + 600),
            );
        }

        if killed {
            self.pending_kills.push((target_entity, fwau));
        }

        AttackResult {
            hit: true,
            target_name: target_name.clone(),
            target_entity: Some(target_entity.index),
            damage: policy.damage,
            target_hp,
            killed,
            player_stamina,
            message: if killed {
                format!("{} fell. Something settled.", target_name)
            } else {
                format!("Strike landed on {}.", target_name)
            },
        }
    }

    pub fn try_interact(&mut self, fwau: FwauId, ledger: &mut EntropyLedger) -> InteractResult {
        let policy = &self.spec.ruleset.verbs.interact;
        if !policy.enabled {
            return InteractResult {
                hit: false,
                target_name: "".into(),
                item_granted: None,
                inventory: vec![],
                message: "Interact disabled by ruleset".into(),
            };
        }

        let player_entity = self.find_avatar_for_fwau(fwau);
        if player_entity.is_none() {
            return fail_interact("No avatar bound");
        }
        let player_entity = player_entity.unwrap();
        let player_pos = self
            .world
            .get(player_entity)
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);

        let (target_entity, target_name, _) = match nearest_interactable(
            &self.world,
            player_pos,
            policy.range_m,
            Some(player_entity),
        ) {
            Some(t) => t,
            None => return fail_interact("Nothing to interact with nearby"),
        };

        let item_id = format!("{}:{}", policy.item_prefix, target_entity.index);
        let conservation = &self.spec.ruleset.conservation.items;
        let clone_tax = self.spec.ruleset.conservation.clone_tax_entropy;
        let now = self.now().0;
        let iuoc_id = self.iuoc_for_fwau(fwau);
        let mut item_granted = None;

        if let Some(rec) = self.world.get_mut(player_entity) {
            let avatar = match rec.avatar.as_mut() {
                Some(a) => a,
                None => return fail_interact("No avatar state"),
            };
            if avatar.dead {
                return fail_interact("You are unbound");
            }

            if conservation == "unique" && avatar.inventory.contains(&item_id) {
                return InteractResult {
                    hit: true,
                    target_name,
                    item_granted: None,
                    inventory: avatar.inventory.clone(),
                    message: "You already carry that echo. Unique conservation.".into(),
                };
            }

            if conservation == "unique" || conservation == "clone-tax" {
                if !avatar.inventory.contains(&item_id) {
                    avatar.inventory.push(item_id.clone());
                    item_granted = Some(item_id.clone());
                    if conservation == "clone-tax" {
                        if let Some(tax) = clone_tax {
                            if let Some(iuoc) = iuoc_id {
                                ledger.enqueue(iuoc, tax, Tick(now + 400), now as u128);
                            }
                        }
                    }
                }
            }
            rec.dirty = true;
            let inventory = avatar.inventory.clone();

            if let Some(iuoc) = iuoc_id {
                ledger.enqueue_consequence(
                    iuoc,
                    &ResolvedAction {
                        id: now as u128 + 1,
                        aid: 0.2,
                        harm: 0.0,
                        ego: 0.0,
                        coerce: 0.0,
                        perf: 0.0,
                    },
                    Tick(now + 800),
                );
            }

            let prop_placed = if self.spec.ruleset.crdt.is_some() {
                let prop_id = format!("prop:{}:{}", policy.item_prefix, target_entity.index);
                let prop = crate::crdt_props::CrdtProp {
                    id: prop_id,
                    kind: policy.item_prefix.clone(),
                    x: player_pos.x,
                    y: player_pos.y,
                    z: 0.0,
                    author_fwau: fwau.0,
                    placed_tick: now,
                };
                self.prop_store.add(prop)
            } else {
                false
            };

            let message = if item_granted.is_some() {
                if prop_placed {
                    format!("Echo and CRDT prop placed near {}.", target_name)
                } else {
                    format!("Echo received from {}.", target_name)
                }
            } else if prop_placed {
                format!("CRDT prop placed near {}.", target_name)
            } else {
                format!("Spoke with {}.", target_name)
            };

            return InteractResult {
                hit: true,
                target_name,
                item_granted,
                inventory,
                message,
            };
        }

        fail_interact("Interact failed")
    }

    pub fn regen_stamina(&mut self) {
        let regen = self.spec.ruleset.dt_ms as f32 / 1000.0 * 8.0;
        for entity in self.world.all_entities() {
            if let Some(rec) = self.world.get_mut(entity) {
                if let Some(avatar) = &mut rec.avatar {
                    if !avatar.dead {
                        avatar.stamina = (avatar.stamina + regen).min(100.0);
                    }
                }
            }
        }
    }

    pub fn process_pending_kills(&mut self) -> Vec<(FwauId, Option<IuocId>)> {
        let mut deaths = Vec::new();
        for (entity, attacker_fwau) in self.pending_kills.drain(..) {
            if let Some(rec) = self.world.get(entity) {
                if rec.fwau_binding.is_some() {
                    let fwau = rec.fwau_binding.as_ref().unwrap().fwau_id;
                    let iuoc = rec.fwau_binding.as_ref().map(|b| b.iuoc_id);
                    deaths.push((fwau, iuoc));
                    continue;
                }
            }
            // AI corpse — despawn after kill
            self.grid.remove_entity(entity);
            self.world.despawn(entity);
            let _ = attacker_fwau;
        }
        deaths
    }
}

fn fail_attack(msg: &str) -> AttackResult {
    AttackResult {
        hit: false,
        target_name: "".into(),
        target_entity: None,
        damage: 0.0,
        target_hp: 0.0,
        killed: false,
        player_stamina: 0.0,
        message: msg.into(),
    }
}

fn fail_interact(msg: &str) -> InteractResult {
    InteractResult {
        hit: false,
        target_name: "".into(),
        item_granted: None,
        inventory: vec![],
        message: msg.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Frame, FrameSpec};
    use crate::iuoc::IuocRegistry;
    use crate::ruleset::Ruleset;
    use crate::types::{FrameId, IuocId};

    #[test]
    fn attack_reduces_ai_hp() {
        let ruleset = Ruleset::pmr_prime();
        let mut frame = Frame::new(FrameSpec {
            id: FrameId(1),
            name: "test".into(),
            ruleset,
            genesis_hash: [0; 32],
        });
        let mut reg = IuocRegistry::new();
        let iuoc = reg.create_soul();
        let fwau = frame
            .bind_player(&mut reg, iuoc, Vec3::new(0.0, 0.0, 0.0))
            .unwrap();
        let ai = frame
            .world
            .spawn_ai_guy("AI-Guy-0", Vec3::new(2.0, 0.0, 0.0), 1);
        let mut ledger = EntropyLedger::new();
        let res = frame.try_attack(fwau, Some(ai), &mut ledger);
        assert!(res.hit);
        assert!(res.target_hp < 100.0);
    }

    #[test]
    fn interact_grants_unique_item() {
        let ruleset = Ruleset::pmr_prime();
        let mut frame = Frame::new(FrameSpec {
            id: FrameId(1),
            name: "test".into(),
            ruleset,
            genesis_hash: [0; 32],
        });
        let mut reg = IuocRegistry::new();
        let iuoc = reg.create_soul();
        let fwau = frame
            .bind_player(&mut reg, iuoc, Vec3::new(0.0, 0.0, 0.0))
            .unwrap();
        frame
            .world
            .spawn_ai_guy("AI-Guy-0", Vec3::new(2.0, 0.0, 0.0), 1);
        let mut ledger = EntropyLedger::new();
        let res = frame.try_interact(fwau, &mut ledger);
        assert!(res.hit);
        assert!(res.item_granted.is_some());
        let again = frame.try_interact(fwau, &mut ledger);
        assert!(again.item_granted.is_none());
    }
}
