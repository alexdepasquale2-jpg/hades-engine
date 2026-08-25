//! The Big Computer (TBC) — MBT-Native MMORPG Engine core library.
//!
//! Implements the consciousness-first simulation substrate described in the TBC Engine Spec:
//! AUM_Core, Δt clocks, EntropyLedger, RenderOracle grid, probability surfaces, IUOC/FWAU identity.

pub mod beam;
pub mod clock;
pub mod ecs;
pub mod frame;
pub mod grid;
pub mod intent;
pub mod iuoc;
pub mod ledger;
pub mod netcode;
pub mod ruleset;
pub mod rww;
pub mod types;

pub mod aum {
    use crate::frame::{Frame, FrameSpec};
    use crate::iuoc::IuocRegistry;
    use crate::ledger::EntropyLedger;
    use crate::ruleset::Ruleset;
    use crate::rww::RwwBus;
    use crate::types::{FrameId, FwauId, IuocId, Vec3};

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
      Self::boot_with_frames(genesis_hash, vec![Ruleset::pmr_prime(), Ruleset::npmr_academy()])
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

    pub fn run_frame_ticks(&mut self, frame_idx: usize, steps: u32) {
      for _ in 0..steps {
        self.frames[frame_idx].step_once(&mut self.ledger);
      }
    }

    pub fn run_all_frames(&mut self, steps: u32) {
      for i in 0..self.frames.len() {
        self.run_frame_ticks(i, steps);
      }
    }
  }
}
