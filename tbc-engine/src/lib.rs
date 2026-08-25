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
pub mod ruleset;
pub mod types;

pub mod aum {
    use crate::frame::{Frame, FrameSpec};
    use crate::iuoc::IuocRegistry;
    use crate::ledger::EntropyLedger;
    use crate::ruleset::Ruleset;
    use crate::types::{FrameId, IuocId};

  /// AUM_Core — single root of authority for one universe deployment.
  pub struct AumCore {
    pub iuoc: IuocRegistry,
    pub ledger: EntropyLedger,
    pub frames: Vec<Frame>,
  }

  pub struct AumConfig {
    pub genesis_hash: [u8; 32],
    pub ruleset: Ruleset,
  }

  impl AumCore {
    pub fn boot(cfg: AumConfig) -> Self {
      let iuoc = IuocRegistry::new();
      let ledger = EntropyLedger::new();
      let frame = Frame::new(
        FrameSpec {
          id: FrameId(1),
          name: cfg.ruleset.title.clone(),
          ruleset: cfg.ruleset,
          genesis_hash: cfg.genesis_hash,
        },
      );
      Self {
        iuoc,
        ledger,
        frames: vec![frame],
      }
    }

    pub fn bind_player(&mut self, iuoc: IuocId, pos: crate::types::Vec3) -> Result<crate::types::FwauId, String> {
      self.frames[0].bind_player(&mut self.iuoc, iuoc, pos)
    }

    pub fn primary_frame(&mut self) -> &mut Frame {
      &mut self.frames[0]
    }

    pub fn run_frame_ticks(&mut self, steps: u32) {
      for _ in 0..steps {
        self.frames[0].step_once(&mut self.ledger);
      }
    }
  }
}
