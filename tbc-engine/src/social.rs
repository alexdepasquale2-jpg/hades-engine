//! M15/M20 — Speak broadcasts and Consent pacts (spec §3 assist).

use crate::aum::AumCore;
use crate::frame::{Frame, RwwOutbox};
use crate::intent::ConsentStamp;
use crate::iuoc::IuocRegistry;
use crate::ledger::{EntropyLedger, ResolvedAction};
use crate::types::{FwauId, IuocId, Tick, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeakEvent {
    pub fwau: u128,
    pub text: String,
    pub tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeakResult {
    pub heard: bool,
    pub listeners: usize,
    pub message: String,
}

impl Frame {
    pub fn try_speak(
        &mut self,
        fwau: FwauId,
        text: &str,
        wire_consent: Option<ConsentStamp>,
        iuoc: &IuocRegistry,
        ledger: &mut EntropyLedger,
    ) -> SpeakResult {
        let ruleset_id = self.spec.ruleset.id.clone();
        let enabled = self.spec.ruleset.verbs.speak.enabled;
        let range_m = self.spec.ruleset.verbs.speak.range_m;
        if !enabled {
            return SpeakResult {
                heard: false,
                listeners: 0,
                message: "Speak disabled by ruleset".into(),
            };
        }

        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 120 {
            return SpeakResult {
                heard: false,
                listeners: 0,
                message: "Speech empty or too long".into(),
            };
        }

        let pos = self
            .find_avatar_for_fwau(fwau)
            .and_then(|e| self.world.get(e))
            .map(|r| r.transform.position)
            .unwrap_or(Vec3::ZERO);

        let tick = self.now().0;
        let helper_iuoc = self.iuoc_for_fwau(fwau);
        if let (Some(stamp), Some(helper)) = (wire_consent.as_ref(), helper_iuoc) {
            if !iuoc.verify_consent_stamp(helper, stamp, tick) {
                return SpeakResult {
                    heard: false,
                    listeners: 0,
                    message: "Speak refused — consent stamp invalid or expired".into(),
                };
            }
        }

        let mut listeners = 0usize;

        for entity in self.world.all_entities() {
            if let Some(rec) = self.world.get(entity) {
                if rec.fwau_binding.as_ref().map(|b| b.fwau_id) == Some(fwau) {
                    continue;
                }
                let dx = rec.transform.position.x - pos.x;
                let dy = rec.transform.position.y - pos.y;
                if (dx * dx + dy * dy).sqrt() <= range_m {
                    listeners += 1;
                }
            }
        }

        self.recent_speaks.push(SpeakEvent {
            fwau: fwau.0,
            text: trimmed.to_string(),
            tick,
        });
        if self.recent_speaks.len() > 32 {
            self.recent_speaks.remove(0);
        }

        if let Some(iuoc_id) = self.iuoc_for_fwau(fwau) {
            ledger.enqueue_consequence(
                iuoc_id,
                &ResolvedAction {
                    id: tick as u128,
                    aid: 0.05,
                    harm: 0.0,
                    ego: 0.05,
                    coerce: 0.0,
                    perf: 0.0,
                },
                Tick(tick + 400),
            );
        }

        self.pending_rww.push(RwwOutbox {
            topic: format!("rww.speak.{}", ruleset_id),
            body: trimmed.as_bytes().to_vec(),
            tick,
            persistent: false,
        });

        SpeakResult {
            heard: listeners > 0,
            listeners,
            message: if listeners > 0 {
                format!("{} nearby minds heard.", listeners)
            } else {
                "Words dissipated. No observers in range.".into()
            },
        }
    }
}

impl AumCore {
    pub fn grant_consent(
        &mut self,
        from: IuocId,
        target: IuocId,
        scope: &str,
        ttl_ticks: u64,
    ) -> Result<String, String> {
        let expires = self
            .frames
            .first()
            .map(|f| f.now().0 + ttl_ticks)
            .unwrap_or(ttl_ticks);
        let pact = format!("{}:{}:{}", scope, target.0, expires);
        self.iuoc
            .add_consent_pact(from, pact.clone())
            .map_err(|e| format!("{:?}", e))?;
        Ok(pact)
    }

    pub fn has_consent(&self, from: IuocId, target: IuocId, scope: &str) -> bool {
        let now = self.frames.first().map(|f| f.now().0).unwrap_or(0);
        self.iuoc.has_consent_pact(from, target, scope, now)
    }
}
