//! The Big Computer (TBC) — MBT-Native MMORPG Engine core library.
//!
//! Implements the consciousness-first simulation substrate described in the TBC Engine Spec:
//! AUM_Core, Δt clocks, EntropyLedger, RenderOracle grid, probability surfaces, IUOC/FWAU identity.

pub mod beam;
pub mod clock;
pub mod crdt_props;
pub mod ecs;
pub mod frame;
pub mod gameplay;
pub mod guardrails;
pub mod grid;
pub mod islands;
pub mod intent;
pub mod iuoc;
pub mod ledger;
pub mod netcode;
pub mod ops;
pub mod persist;
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
    use crate::ruleset::{Ruleset, RulesetRegistry};
    use crate::rww::RwwBus;
    use crate::shard::{ShardBounds, ShardCrossEvent, ShardNodeConfig, ShardNodeStatus};
    use crate::types::{FrameId, FwauId, IuocId, QualityScalar, Tick, Vec3};
    use std::collections::HashSet;

  /// AUM_Core — single root of authority for one universe deployment.
  pub struct AumCore {
    pub iuoc: IuocRegistry,
    pub ledger: EntropyLedger,
    pub frames: Vec<Frame>,
    pub rww: RwwBus,
    pub node: ShardNodeConfig,
    processed_cross_events: HashSet<String>,
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

    /// M9: cluster boot with SQLite soul archive hydration.
    pub fn boot_cluster_with_archive(
      genesis_hash: [u8; 32],
      archive: crate::persist::SoulArchive,
    ) -> Result<Self, crate::persist::PersistError> {
      Self::boot_node_with_archive(genesis_hash, archive, ShardNodeConfig::cluster())
    }

    /// M11: boot cluster or single distributed shard node.
    pub fn boot_node_with_archive(
      genesis_hash: [u8; 32],
      archive: crate::persist::SoulArchive,
      node: ShardNodeConfig,
    ) -> Result<Self, crate::persist::PersistError> {
      let iuoc = IuocRegistry::with_archive(archive)?;
      Ok(Self::boot_node_inner(
        genesis_hash,
        iuoc,
        node,
        RwwBus::open_from_env(),
      ))
    }

    /// M11 tests: boot a shard node with a shared RWW bus (in-memory clone or NATS).
    pub fn boot_node_with_archive_rww(
      genesis_hash: [u8; 32],
      archive: crate::persist::SoulArchive,
      node: ShardNodeConfig,
      rww: RwwBus,
    ) -> Result<Self, crate::persist::PersistError> {
      let iuoc = IuocRegistry::with_archive(archive)?;
      Ok(Self::boot_node_inner(genesis_hash, iuoc, node, rww))
    }

    fn frame_for_shard(genesis_hash: [u8; 32], shard_id: u32) -> Frame {
      let bounds = ShardBounds::for_id(shard_id).unwrap_or_else(ShardBounds::shard_00);
      let reg = RulesetRegistry::boot_defaults();
      let pmr = reg
        .get("pmr.v1")
        .cloned()
        .unwrap_or_else(Ruleset::pmr_prime);
      Frame::new_with_shard(
        FrameSpec {
          id: FrameId(shard_id + 1),
          name: format!("PMR {}", bounds.name),
          ruleset: pmr,
          genesis_hash,
        },
        Some(bounds),
      )
    }

    fn cluster_frames(genesis_hash: [u8; 32]) -> Vec<Frame> {
      let reg = RulesetRegistry::boot_defaults();
      let pmr = reg
        .get("pmr.v1")
        .cloned()
        .unwrap_or_else(Ruleset::pmr_prime);
      let npmr = reg
        .get("npmr.academy.v1")
        .cloned()
        .unwrap_or_else(Ruleset::npmr_academy);
      let dream = reg
        .get("npmr.dream.v1")
        .cloned()
        .unwrap_or_else(Ruleset::npmr_dream);
      vec![
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
        Frame::new(FrameSpec {
          id: FrameId(4),
          name: dream.title.clone(),
          ruleset: dream,
          genesis_hash,
        }),
      ]
    }

    fn boot_node_inner(
      genesis_hash: [u8; 32],
      iuoc: IuocRegistry,
      node: ShardNodeConfig,
      rww: RwwBus,
    ) -> Self {
      let ledger = EntropyLedger::new();
      let frames = match node.shard_id {
        Some(id) => vec![Self::frame_for_shard(genesis_hash, id)],
        None => Self::cluster_frames(genesis_hash),
      };

      Self {
        iuoc,
        ledger,
        frames,
        rww,
        node,
        processed_cross_events: HashSet::new(),
      }
    }

    pub fn node_status(&self) -> ShardNodeStatus {
      ShardNodeStatus {
        mode: self.node.label().into(),
        shard_id: self.node.shard_id,
        distributed: self.node.distributed,
        frame_count: self.frames.len(),
      }
    }

    pub fn ops_snapshot(&self, active_sessions: usize) -> crate::ops::OpsSnapshot {
      crate::ops::OpsSnapshot::from_aum(self, active_sessions)
    }

    /// M7 legacy cluster boot (in-memory souls).
    pub fn boot_cluster(genesis_hash: [u8; 32]) -> Self {
      Self::boot_node_inner(
        genesis_hash,
        IuocRegistry::new(),
        ShardNodeConfig::cluster(),
        RwwBus::open_from_env(),
      )
    }

    fn boot_with_frames(genesis_hash: [u8; 32], rulesets: Vec<Ruleset>) -> Self {
      let iuoc = IuocRegistry::new();
      let ledger = EntropyLedger::new();
      let rww = RwwBus::open_from_env();
      let node = ShardNodeConfig::cluster();
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
        node,
        processed_cross_events: HashSet::new(),
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
      if from_idx >= self.frames.len() || to_idx >= self.frames.len() {
        return Err("frame index out of range".into());
      }
      let to_ruleset = self.frames[to_idx].spec.ruleset.id.clone();
      if !self.frames[from_idx]
        .spec
        .ruleset
        .handoff
        .allowed_targets
        .contains(&to_ruleset)
      {
        return Err(format!(
          "ruleset {} forbids handoff to {}",
          self.frames[from_idx].spec.ruleset.id,
          to_ruleset
        ));
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
          if self.node.distributed {
            let frame = &self.frames[i];
            let iuoc = frame.iuoc_for_fwau(fwau);
            let pos = frame
              .find_avatar_for_fwau(fwau)
              .and_then(|e| frame.world.get(e))
              .map(|r| r.transform.position)
              .unwrap_or(Vec3::new(0.0, 0.0, 0.0));
            let from_shard = frame.shard_bounds.as_ref().map(|s| s.id).unwrap_or(0);
            let tick = frame.now().0;
            let ruleset = frame.spec.ruleset.id.clone();
            if let Some(iuoc_id) = iuoc {
              let event = ShardCrossEvent::new(
                iuoc_id.0,
                fwau.0,
                from_shard,
                target_shard,
                pos,
                tick,
                &ruleset,
              );
              self.rww.publish_shard_cross(&event);
              self.frames[i].unbind_fwau(fwau);
              self.iuoc.release_live_fwau(fwau);
              completed.push((fwau, FwauId(0), i));
            }
            continue;
          }

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

    /// Consume inbound `rww.shard.cross` events on distributed shard nodes (M11).
    pub fn process_inbound_shard_crosses(&mut self) -> Vec<(FwauId, IuocId)> {
      if !self.node.distributed || self.node.shard_id.is_none() {
        return vec![];
      }
      let my_shard = self.node.shard_id.unwrap();
      let msgs = self.rww.recent("rww.shard.cross", 32);
      let mut bound = Vec::new();

      for msg in msgs {
        if self.processed_cross_events.contains(&msg.id) {
          continue;
        }
        let event: ShardCrossEvent = match serde_json::from_slice(&msg.payload) {
          Ok(e) => e,
          Err(_) => continue,
        };
        if event.to_shard != my_shard {
          continue;
        }
        let iuoc = IuocId(event.iuoc);
        if self.iuoc.refresh_soul_from_archive(iuoc).unwrap_or(false) == false {
          continue;
        }
        if self
          .iuoc
          .get(iuoc)
          .map(|s| s.bound_fwau.is_some())
          .unwrap_or(false)
        {
          self.processed_cross_events.insert(msg.id.clone());
          continue;
        }
        let pos = Vec3::new(event.x, event.y, event.z);
        if let Ok(fwau) = self.bind_player(0, iuoc, pos) {
          bound.push((fwau, iuoc));
          self.processed_cross_events.insert(msg.id.clone());
          tracing::info!(
            "Inbound shard cross: iuoc={} fwau={} from shard-{}",
            event.iuoc,
            fwau.0,
            event.from_shard
          );
        }
      }

      bound
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

      let frame_idx = if self.node.distributed {
        let my_shard = self.node.shard_id.unwrap_or(0);
        if offer.start_shard != my_shard {
          return Err("reincarnation offer targets another shard node".into());
        }
        0
      } else {
        offer.start_shard as usize
      };
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

    pub fn archive_stats(&self) -> Option<crate::persist::ArchiveStats> {
      self.iuoc.archive_stats()
    }

    pub fn resume_soul(
      &mut self,
      iuoc: IuocId,
      frame_idx: usize,
      pos: Vec3,
    ) -> Result<FwauId, String> {
      let soul = self.iuoc.get(iuoc).ok_or("IUOC not found in archive")?;
      if soul.bound_fwau.is_some() {
        return Err("IUOC already bound to a live FWAU".into());
      }
      if frame_idx >= self.frames.len() {
        return Err("frame index out of range".into());
      }
      self.bind_player(frame_idx, iuoc, pos)
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
        self.process_player_deaths(i);
      }
      self.process_shard_handoffs();
    }

    pub fn attack(
      &mut self,
      frame_idx: usize,
      fwau: FwauId,
      target: Option<crate::types::Entity>,
    ) -> crate::gameplay::AttackResult {
      self.frames[frame_idx].try_attack(fwau, target, &mut self.ledger)
    }

    pub fn interact(
      &mut self,
      frame_idx: usize,
      fwau: FwauId,
    ) -> crate::gameplay::InteractResult {
      self.frames[frame_idx].try_interact(fwau, &mut self.ledger)
    }

    /// Auto-unbind FWAUs when ruleset death policy allows (M13).
    pub fn process_player_deaths(&mut self, frame_idx: usize) -> Vec<FwauId> {
      let unbind = self.frames[frame_idx].spec.ruleset.death.unbind;
      let deaths = self.frames[frame_idx].process_pending_kills();
      let mut unbound = Vec::new();
      for (fwau, iuoc) in deaths {
        if iuoc.is_some() && unbind {
          self.unbind_death(fwau, frame_idx);
          unbound.push(fwau);
        }
      }
      unbound
    }
  }
}
