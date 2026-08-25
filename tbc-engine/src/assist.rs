//! M16/M20 — consent-gated assist intents (spec §3).

use crate::frame::{Frame, RwwOutbox};
use crate::gameplay::{distance2, nearest_interactable};
use crate::intent::ConsentStamp;
use crate::iuoc::IuocRegistry;
use crate::ledger::{EntropyLedger, ResolvedAction};
use crate::types::{Entity, FwauId, Tick, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistResult {
    pub hit: bool,
    pub target_name: String,
    pub target_entity: Option<u32>,
    pub healed: f32,
    pub target_hp: f32,
    pub player_stamina: f32,
    pub consent_verified: bool,
    pub message: String,
}

impl Frame {
  pub fn try_assist(
        &mut self,
        fwau: FwauId,
        target: Option<Entity>,
        wire_consent: Option<ConsentStamp>,
        iuoc: &IuocRegistry,
        ledger: &mut EntropyLedger,
    ) -> AssistResult {
        let policy = self.spec.ruleset.verbs.assist.clone();
        if !policy.enabled {
            return fail_assist("Assist disabled by ruleset");
        }

        let player_entity = self.find_avatar_for_fwau(fwau);
        if player_entity.is_none() {
            return fail_assist("No avatar bound");
        }
        let player_entity = player_entity.unwrap();
        let player_pos = self
            .world
            .get(player_entity)
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);
        let helper_iuoc = self.iuoc_for_fwau(fwau);

        let (target_entity, target_name) = if let Some(t) = target {
            let name = self
                .world
                .get(t)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            (t, name)
        } else {
            match nearest_interactable(
                &self.world,
                player_pos,
                policy.range_m,
                Some(player_entity),
            ) {
                Some((e, name, _)) => (e, name),
                None => return fail_assist("No assist target in range"),
            }
        };

        let target_pos = self
            .world
            .get(target_entity)
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);
        if distance2(player_pos, target_pos) > policy.range_m {
            return fail_assist("Target out of range");
        }

        let target_rec = self.world.get(target_entity);
        let is_ai = target_rec.map(|r| r.brain.is_some()).unwrap_or(false);
        let target_iuoc = target_rec
            .and_then(|r| r.fwau_binding.as_ref())
            .map(|b| b.iuoc_id);
        let target_hp_before = target_rec
            .and_then(|r| r.avatar.as_ref())
            .map(|a| a.hp)
            .unwrap_or(0.0);

        let now_tick = self.now().0;
        let consent_verified = if is_ai && policy.ai_practice {
            true
        } else if let Some(helper_id) = helper_iuoc {
            if let Some(stamp) = wire_consent.as_ref() {
                iuoc.verify_consent_stamp(helper_id, stamp, now_tick)
            } else if let Some(target_id) = target_iuoc {
                iuoc.has_consent_pact(target_id, helper_id, "assist", now_tick)
            } else {
                false
            }
        } else {
            false
        };

        if !consent_verified {
            return AssistResult {
                hit: false,
                target_name,
                target_entity: Some(target_entity.index),
                healed: 0.0,
                target_hp: target_hp_before,
                player_stamina: 0.0,
                consent_verified: false,
                message: "Assist refused — consent pact required from target soul".into(),
            };
        }

        let mut player_stamina = 0.0;
        if let Some(rec) = self.world.get_mut(player_entity) {
            let avatar = match rec.avatar.as_mut() {
                Some(a) => a,
                None => return fail_assist("No avatar state"),
            };
            if avatar.dead {
                return fail_assist("You are unbound");
            }
            if avatar.stamina < policy.stamina_cost {
                return fail_assist("Stamina exhausted");
            }
            avatar.stamina -= policy.stamina_cost;
            player_stamina = avatar.stamina;
            rec.dirty = true;
        }

        let now = self.now().0;
        let mut healed = 0.0;
        let mut target_hp = 0.0;

        if let Some(rec) = self.world.get_mut(target_entity) {
            if let Some(avatar) = &mut rec.avatar {
                let before = avatar.hp;
                avatar.hp = (avatar.hp + policy.heal_amount).min(100.0);
                healed = avatar.hp - before;
                target_hp = avatar.hp;
                if avatar.dead && avatar.hp > 0.0 {
                    avatar.dead = false;
                }
                rec.dirty = true;
            } else {
                return fail_assist("Target cannot receive assist");
            }
        } else {
            return fail_assist("Target missing");
        }

        if let Some(iuoc_id) = helper_iuoc {
            ledger.enqueue_consequence(
                iuoc_id,
                &ResolvedAction {
                    id: now as u128,
                    aid: policy.aid_entropy,
                    harm: 0.0,
                    ego: 0.0,
                    coerce: 0.0,
                    perf: 0.1,
                },
                Tick(now + 500),
            );
        }

        self.pending_rww.push(RwwOutbox {
            topic: format!("rww.assist.{}", self.spec.ruleset.id),
            body: format!("{} healed {:.0}", target_name, healed).into_bytes(),
            tick: now,
            persistent: false,
        });

        AssistResult {
            hit: true,
            target_name: target_name.clone(),
            target_entity: Some(target_entity.index),
            healed,
            target_hp,
            player_stamina,
            consent_verified: true,
            message: if healed > 0.0 {
                format!(
                    "Assist landed on {} (+{:.0} vitality).",
                    target_name, healed
                )
            } else {
                format!("{} already at full vitality.", target_name)
            },
        }
    }
}

pub fn fail_assist(msg: &str) -> AssistResult {
    AssistResult {
        hit: false,
        target_name: "".into(),
        target_entity: None,
        healed: 0.0,
        target_hp: 0.0,
        player_stamina: 0.0,
        consent_verified: false,
        message: msg.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aum::AumCore;
    use crate::types::Vec3;

    #[test]
    fn npmr_ai_practice_assist_without_consent() {
        let genesis = *blake3::hash(b"m16-assist-npmr").as_bytes();
        let mut aum = AumCore::boot_cluster(genesis);
        aum.frames[2].spawn_demo_world(4);
        let iuoc = aum.iuoc.create_soul();
        let fwau = aum.bind_player(2, iuoc, Vec3::new(50.0, 0.0, 0.0)).unwrap();
        let res = aum.assist(2, fwau, None, None);
        assert!(res.hit);
        assert!(res.consent_verified);
    }

    #[test]
    fn pmr_assist_requires_consent() {
        let genesis = *blake3::hash(b"m16-assist-pmr").as_bytes();
        let mut aum = AumCore::boot_cluster(genesis);
        aum.frames[0].spawn_demo_world(4);
        let helper = aum.iuoc.create_soul();
        let target = aum.iuoc.create_soul();
        let target_fwau = aum
            .bind_player(0, target, Vec3::new(-175.0, 0.0, 0.0))
            .unwrap();
        let helper_fwau = aum
            .bind_player(0, helper, Vec3::new(-174.0, 0.0, 0.0))
            .unwrap();
        let target_entity = aum.frames[0]
            .find_avatar_for_fwau(target_fwau)
            .expect("target avatar");
        if let Some(rec) = aum.frames[0].world.get_mut(target_entity) {
            if let Some(a) = &mut rec.avatar {
                a.hp = 40.0;
            }
        }
        let denied = aum.assist(0, helper_fwau, Some(target_entity), None);
        assert!(!denied.hit);
        assert!(!denied.consent_verified);
        aum.grant_consent(target, helper, "assist", 50_000)
            .expect("grant");
        let expires = aum.frames[0].now().0 + 50_000;
        let stamp = crate::intent::ConsentStamp {
            target: target.0,
            scope: "assist".into(),
            expires_tick: expires,
        };
        let ok = aum.assist(0, helper_fwau, Some(target_entity), Some(stamp));
        assert!(ok.hit);
        assert!(ok.consent_verified);
        assert!(ok.healed > 0.0);
    }
}
