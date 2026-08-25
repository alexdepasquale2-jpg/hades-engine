//! The Big Computer (TBC) — MBT-Native MMORPG Engine core library.
//!
//! Implements the consciousness-first simulation substrate described in the TBC Engine Spec:
//! AUM_Core, Δt clocks, EntropyLedger, RenderOracle grid, probability surfaces, IUOC/FWAU identity.

pub mod beam;
pub mod clock;
pub mod ecs;
pub mod frame;
pub mod guardrails;
pub mod grid;
pub mod islands;
pub mod intent;
pub mod iuoc;
pub mod ledger;
pub mod netcode;
pub mod planner;
pub mod ruleset;
pub mod rww;
pub mod shard;
pub mod transport;
pub mod types;

pub mod aum {
    use crate::frame::{Frame, FrameSpec};
    use crate::iuoc::IuocRegistry;
    use crate::ledger::EntropyLedger;
    use crate::planner::{rank_offers, ReincarnationOffer};
    use crate::ruleset::Ruleset;
    use crate::rww::RwwBus;
    use crate::shard::ShardBounds;
    use crate::types::{FrameId, FwauId, IuocId, QualityScalar, Tick, Vec3};

  /// AUM_Core — single root of authority for one universe deployment.
  pub struct AumCore {
    pub iuoc: IuocRegistry,
    pub ledger: EntropyLedger,
    pub frames: Vec<Frame>,
    pub rww: RwwBus,
  }

  pub struct AumConfig {
    pub genesis_hash: [u8; 32],
    pub ruleset: Ruleset,
  }

  impl AumCore {
    pub fn boot(cfg: AumConfig) -> Self {
      Self::boot_with_frames(cfg.genesis_hash, vec![cfg.ruleset])
    }

    pub fn boot_full(genesis_hash: [u8; 32]) -> Self {
      Self::boot_cluster(genesis_hash)
    }

    /// M7: PMR shard-00, shard-01, NPMR-Academy
    pub fn boot_cluster(genesis_hash: [u8; 32]) -> Self {
      let iuoc = IuocRegistry::new();
      let ledger = EntropyLedger::new();
      let rww = RwwBus::new();
      let pmr = Ruleset::pmr_prime();
      let npmr = Ruleset::npmr_academy();

      let frames = vec![
        Frame::new_with_shard(
          FrameSpec {
            id: FrameId(1),
            name: "PMR shard-00".into(),
            ruleset: pmr.clone(),
            genesis_hash,
          },
          Some(ShardBounds::shard_00()),
        ),
        Frame::new_with_shard(
          FrameSpec {
            id: FrameId(2),
            name: "PMR shard-01".into(),
            ruleset: pmr,
            genesis_hash,
          },
          Some(ShardBounds::shard_01()),
        ),
        Frame::new(FrameSpec {
          id: FrameId(3),
          name: npmr.title.clone(),
          ruleset: npmr,
          genesis_hash,
        }),
      ];

      Self {
        iuoc,
        ledger,
        frames,
        rww,
      }
    }

    fn boot_with_frames(genesis_hash: [u8; 32], rulesets: Vec<Ruleset>) -> Self {
      let iuoc = IuocRegistry::new();
      let ledger = EntropyLedger::new();
      let rww = RwwBus::new();
      let frames = rulesets
        .into_iter()
        .enumerate()
        .map(|(i, ruleset)| {
          Frame::new(FrameSpec {
            id: FrameId(i as u32 + 1),
            name: ruleset.title.clone(),
            ruleset,
            genesis_hash,
          })
        })
        .collect();
      Self {
        iuoc,
        ledger,
        frames,
        rww,
      }
    }

    pub fn bind_player(
      &mut self,
      frame_idx: usize,
      iuoc: IuocId,
      pos: Vec3,
    ) -> Result<FwauId, String> {
      let fwau = self.frames[frame_idx].bind_player(&mut self.iuoc, iuoc, pos)?;
      self.rww.publish_fwau_bound(
        fwau.0,
        &self.frames[frame_idx].spec.ruleset.id,
        self.frames[frame_idx].now().0,
      );
      Ok(fwau)
    }

    pub fn handoff_fwau(
      &mut self,
      fwau: FwauId,
      from_idx: usize,
      to_idx: usize,
    ) -> Result<(), String> {
      if from_idx == to_idx {
        return Ok(());
      }
      let entity = self.frames[from_idx]
        .find_avatar_for_fwau(fwau)
        .ok_or("avatar not found in source frame")?;
      let rec = self.frames[from_idx]
        .world
        .get(entity)
        .ok_or("entity missing")?;
      let pos = rec.transform.position;
      let iuoc = rec
        .fwau_binding
        .as_ref()
        .map(|b| b.iuoc_id)
        .ok_or("not a player")?;

      self.frames[from_idx].unbind_fwau(fwau);
      let new_fwau = self.frames[to_idx].bind_player(&mut self.iuoc, iuoc, pos)?;
      self.rww.publish_handoff(
        new_fwau.0,
        &self.frames[from_idx].spec.ruleset.id,
        &self.frames[to_idx].spec.ruleset.id,
        self.frames[to_idx].now().0,
      );
      Ok(())
    }

    pub fn psi_past_own(&self, iuoc: IuocId) -> Vec<String> {
      self
        .iuoc
        .packets_for(iuoc)
        .iter()
        .take(5)
        .map(|p| format!("{}: {}", p.death_cause, p.summary))
        .collect()
    }

    pub fn primary_frame(&mut self) -> &mut Frame {
      &mut self.frames[0]
    }

    /// Returns (old_fwau, new_fwau, target_frame_idx) for each completed shard crossing.
    pub fn process_shard_handoffs(&mut self) -> Vec<(FwauId, FwauId, usize)> {
      let mut completed = Vec::new();
      for i in 0..self.frames.len() {
        let handoffs: Vec<(FwauId, u32)> = self.frames[i].pending_shard_handoffs.drain(..).collect();
        for (fwau, target_shard) in handoffs {
          let to_idx = self
            .frames
            .iter()
            .position(|f| f.shard_bounds.as_ref().map(|s| s.id) == Some(target_shard));
          if let Some(to) = to_idx {
            if to != i {
              let iuoc = self.frames[i].iuoc_for_fwau(fwau);
              if self.handoff_fwau(fwau, i, to).is_ok() {
                let new_fwau = iuoc
                  .and_then(|id| self.iuoc.get(id).and_then(|s| s.bound_fwau))
                  .unwrap_or(fwau);
                completed.push((fwau, new_fwau, to));
              }
            }
          }
        }
      }
      completed
    }

    pub fn accept_reincarnation(
      &mut self,
      iuoc: IuocId,
      template_id: &str,
    ) -> Result<(FwauId, usize), String> {
      if self
        .iuoc
        .get(iuoc)
        .map(|s| s.bound_fwau.is_some())
        .unwrap_or(false)
      {
        return Err("IUOC still bound to a FWAU".into());
      }

      let offer = self
        .reincarnation_offers(iuoc)
        .into_iter()
        .find(|o| o.template_id == template_id)
        .or_else(|| {
          crate::planner::starter_pool()
            .into_iter()
            .find(|t| t.id == template_id)
            .map(|t| ReincarnationOffer {
              template_id: t.id,
              title: t.title,
              situation: t.situation,
              faction: t.faction,
              start_shard: t.start_shard,
              expected_delta_s: -0.03,
              score: 0.5,
              odds_label: "starter".into(),
            })
        })
        .ok_or_else(|| format!("unknown template: {}", template_id))?;

      let frame_idx = offer.start_shard as usize;
      if frame_idx >= self.frames.len() {
        return Err("shard frame missing".into());
      }

      let pos = if offer.start_shard == 0 {
        Vec3::new(-80.0, 0.0, 0.0)
      } else {
        Vec3::new(80.0, 0.0, 0.0)
      };

      let fwau = self.bind_player(frame_idx, iuoc, pos)?;
      Ok((fwau, frame_idx))
    }

    pub fn reincarnation_offers(&self, iuoc: IuocId) -> Vec<ReincarnationOffer> {
      let soul = self.iuoc.get(iuoc);
      let quality = soul.map(|s| s.quality).unwrap_or(QualityScalar::INITIAL);
      let inc = soul.map(|s| s.incarnations).unwrap_or(0);
      let seen: Vec<String> = self
        .iuoc
        .packets_for(iuoc)
        .iter()
        .map(|p| p.summary.clone())
        .collect();
      rank_offers(quality, inc, &seen)
    }

    pub fn unbind_death(&mut self, fwau: FwauId, frame_idx: usize) -> Option<crate::iuoc::ExperiencePacket> {
      let frame_id = self.frames.get(frame_idx).map(|f| f.spec.id).unwrap_or(FrameId(0));
      let tick = self.frames.get(frame_idx).map(|f| f.now()).unwrap_or(Tick(0));
      let iuoc = self.frames.get(frame_idx).and_then(|frame| {
        frame
          .find_avatar_for_fwau(fwau)
          .and_then(|e| frame.world.get(e))
          .and_then(|r| r.fwau_binding.as_ref())
          .map(|b| b.iuoc_id)
      });

      if let Some(frame) = self.frames.get_mut(frame_idx) {
        frame.unbind_fwau(fwau);
      }

      if let Some(iuoc) = iuoc {
        let q = self.ledger.get(iuoc);
        self.iuoc.merge_fwau(fwau, frame_id, tick, "death", q)
      } else {
        None
      }
    }

    pub fn run_frame_ticks(&mut self, frame_idx: usize, steps: u32) {
      for _ in 0..steps {
        self.frames[frame_idx].step_once(&mut self.ledger);
      }
    }

    pub fn run_all_frames(&mut self, steps: u32) {
      for i in 0..self.frames.len() {
        self.run_frame_ticks(i, steps);
      }
      self.process_shard_handoffs();
    }
  }
}
