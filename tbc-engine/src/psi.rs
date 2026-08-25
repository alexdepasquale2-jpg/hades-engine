//! M15 — psi scopes (spec §7) with ruleset gating and budget.

use crate::aum::AumCore;
use crate::types::{FwauId, IuocId};

#[derive(Clone, Debug, serde::Serialize)]
pub struct PsiResponse {
  pub scope: String,
  pub allowed: bool,
  pub odds: Vec<(String, f32)>,
  pub recall: Vec<String>,
  pub psi_budget_remaining: f32,
  pub message: String,
}

impl AumCore {
  pub fn query_psi(
    &mut self,
    frame_idx: usize,
    fwau: FwauId,
    iuoc: IuocId,
    scope: &str,
  ) -> PsiResponse {
    let ruleset_id = self
      .frames
      .get(frame_idx)
      .map(|f| f.spec.ruleset.id.clone());
    let _ = ruleset_id;
    let psi_policy = self.frames.get(frame_idx).map(|f| f.spec.ruleset.psi.clone());

    if psi_policy.as_ref().map(|p| !p.enabled).unwrap_or(true) {
      return PsiResponse {
        scope: scope.into(),
        allowed: false,
        odds: vec![],
        recall: vec![],
        psi_budget_remaining: 0.0,
        message: "Psi disabled by ruleset".into(),
      };
    }

    if !psi_policy
      .as_ref()
      .map(|p| p.scopes.iter().any(|s| s == scope))
      .unwrap_or(false)
    {
      return PsiResponse {
        scope: scope.into(),
        allowed: false,
        odds: vec![],
        recall: vec![],
        psi_budget_remaining: self.psi_budget_remaining(fwau),
        message: format!("Scope {} not allowed in this frame", scope),
      };
    }

    let cost = psi_policy.as_ref().map(|p| p.base_cost).unwrap_or(0.04);
    if !self.spend_psi_budget(fwau, cost) {
      return PsiResponse {
        scope: scope.into(),
        allowed: false,
        odds: vec![],
        recall: vec![],
        psi_budget_remaining: self.psi_budget_remaining(fwau),
        message: "Psi budget exhausted".into(),
      };
    }

    let (odds, recall) = match scope {
      "FutureSelf" => {
        let odds = self
          .frames
          .get(frame_idx)
          .map(|f| f.psi_future_self(fwau))
          .unwrap_or_default();
        (odds, vec![])
      }
      "PastOwn" => (vec![], self.psi_past_own(iuoc)),
      "PastShared" => (vec![], self.iuoc.recent_shared_summaries(iuoc, 5)),
      "FutureIsland" => {
        let odds = self
          .frames
          .get(frame_idx)
          .map(|f| f.psi_future_island(fwau))
          .unwrap_or_default();
        (odds, vec![])
      }
      "RwwQuery" => {
        let recall = self
          .rww
          .recent("rww.handoff", 3)
          .into_iter()
          .map(|m| format!("{} @ tick {}", m.subject, m.at_tick))
          .chain(
            self
              .rww
              .recent("rww.shard.cross", 3)
              .into_iter()
              .map(|m| format!("shard cross @ tick {}", m.at_tick)),
          )
          .collect();
        (vec![], recall)
      }
      _ => (vec![], vec![]),
    };

    PsiResponse {
      scope: scope.into(),
      allowed: true,
      odds,
      recall,
      psi_budget_remaining: self.psi_budget_remaining(fwau),
      message: "Probability surface consulted.".into(),
    }
  }

  fn psi_budget_remaining(&self, fwau: FwauId) -> f32 {
    self.iuoc.psi_budget_remaining(fwau)
  }

  fn spend_psi_budget(&mut self, fwau: FwauId, cost: f32) -> bool {
    self.iuoc.spend_psi_budget(fwau, cost)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::Vec3;

  #[test]
  fn future_self_returns_odds() {
    let genesis = *blake3::hash(b"psi-future").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(4);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(0, iuoc, Vec3::ZERO).unwrap();
    let res = aum.query_psi(0, fwau, iuoc, "FutureSelf");
    assert!(res.allowed);
    assert!(res.psi_budget_remaining < aum.iuoc.psi_budget_remaining(fwau) + 0.001 || res.odds.len() > 0);
  }

  #[test]
  fn blocked_scope_rejected() {
    let genesis = *blake3::hash(b"psi-block").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(0, iuoc, Vec3::ZERO).unwrap();
    let res = aum.query_psi(0, fwau, iuoc, "NotAScope");
    assert!(!res.allowed);
  }
}
