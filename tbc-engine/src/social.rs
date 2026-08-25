//! M15 — Speak broadcasts and Consent pacts (spec §3 assist).

use crate::aum::AumCore;
use crate::ledger::ResolvedAction;
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

impl AumCore {
  pub fn speak(&mut self, frame_idx: usize, fwau: FwauId, text: &str) -> SpeakResult {
    let ruleset_id = self.frames[frame_idx].spec.ruleset.id.clone();
    let enabled = self.frames[frame_idx].spec.ruleset.verbs.speak.enabled;
    let range_m = self.frames[frame_idx].spec.ruleset.verbs.speak.range_m;
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

    let pos = {
      let frame = &self.frames[frame_idx];
      frame
        .find_avatar_for_fwau(fwau)
        .and_then(|e| frame.world.get(e))
        .map(|r| r.transform.position)
        .unwrap_or(Vec3::ZERO)
    };

    let frame = &mut self.frames[frame_idx];
    let tick = frame.now().0;
    let mut listeners = 0usize;

    for entity in frame.world.all_entities() {
      if let Some(rec) = frame.world.get(entity) {
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

    frame.recent_speaks.push(SpeakEvent {
      fwau: fwau.0,
      text: trimmed.to_string(),
      tick,
    });
    if frame.recent_speaks.len() > 32 {
      frame.recent_speaks.remove(0);
    }

    if let Some(iuoc) = frame.iuoc_for_fwau(fwau) {
      self.ledger.enqueue_consequence(
        iuoc,
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

    self.rww.publish(
      &format!("rww.speak.{}", ruleset_id),
      trimmed.as_bytes(),
      tick,
      false,
    );

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
    self
      .iuoc
      .add_consent_pact(from, pact.clone())
      .map_err(|e| format!("{:?}", e))?;
    Ok(pact)
  }

  pub fn has_consent(&self, from: IuocId, target: IuocId, scope: &str) -> bool {
    let now = self.frames.first().map(|f| f.now().0).unwrap_or(0);
    self
      .iuoc
      .get(from)
      .map(|s| {
        s.consent
          .pacts
          .iter()
          .any(|p| p.starts_with(&format!("{}:{}", scope, target.0)) && {
            p.rsplit(':').next().and_then(|t| t.parse().ok()).unwrap_or(0) >= now
          })
      })
      .unwrap_or(false)
  }
}
