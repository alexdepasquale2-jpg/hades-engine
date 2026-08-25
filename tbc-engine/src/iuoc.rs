use crate::types::{
  AvatarId, FwauId, FrameId, IuocId, PacketId, QualityScalar, Tick,
};
use crate::persist::{ArchiveStats, PersistError, SoulArchive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConsentGraph {
  pub pacts: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReincarnationPrefs {
  pub preferred_factions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IUOC {
  pub id: IuocId,
  pub quality: QualityScalar,
  pub incarnations: u32,
  pub bound_fwau: Option<FwauId>,
  pub consent: ConsentGraph,
  pub prefs: ReincarnationPrefs,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryClearance {
  IncarnationOnly,
  PastOwn,
  PastShared,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FWAU {
  pub id: FwauId,
  pub iuoc_id: IuocId,
  pub avatar_id: EntityRef,
  pub quality_snapshot: QualityScalar,
  pub intent_buf: IntentQueueSnapshot,
  pub psi_budget: f32,
  pub bound_at: Tick,
  pub memory_clearance: MemoryClearance,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EntityRef {
  pub index: u32,
  pub generation: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct IntentQueueSnapshot {
  pub len: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperiencePacket {
  pub id: PacketId,
  pub iuoc: IuocId,
  pub avatar: AvatarId,
  pub frame: FrameId,
  pub bound_at: Tick,
  pub unbound_at: Tick,
  pub summary: String,
  pub death_cause: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnbindReason {
  Death,
  Logout,
  Disconnect,
  Superseded,
}

pub struct IuocRegistry {
  souls: HashMap<IuocId, IUOC>,
  fwau_sessions: HashMap<FwauId, FWAU>,
  packets: HashMap<IuocId, Vec<ExperiencePacket>>,
  archive: Option<SoulArchive>,
}

impl IuocRegistry {
  pub fn new() -> Self {
    Self {
      souls: HashMap::new(),
      fwau_sessions: HashMap::new(),
      packets: HashMap::new(),
      archive: None,
    }
  }

  pub fn with_archive(archive: SoulArchive) -> Result<Self, PersistError> {
    let mut reg = Self::new();
    reg.archive = Some(archive);
    reg.hydrate()?;
    Ok(reg)
  }

  pub fn attach_archive(&mut self, archive: SoulArchive) -> Result<(), PersistError> {
    self.archive = Some(archive);
    self.hydrate()?;
    Ok(())
  }

  pub fn archive_stats(&self) -> Option<ArchiveStats> {
    self
      .archive
      .as_ref()
      .and_then(|a| a.stats().ok())
  }

  fn hydrate(&mut self) -> Result<(), PersistError> {
    if let Some(archive) = self.archive.as_ref() {
      for soul in archive.load_souls()? {
        self.souls.insert(soul.id, soul);
      }
      for (iuoc, list) in archive.load_packets()? {
        self.packets.insert(iuoc, list);
      }
    }
    Ok(())
  }

  fn persist_soul(&self, iuoc: IuocId) {
    if let Some(archive) = &self.archive {
      if let Some(soul) = self.souls.get(&iuoc) {
        archive.upsert_soul(soul).ok();
      }
    }
  }

  fn persist_packet(&self, packet: &ExperiencePacket) {
    if let Some(archive) = &self.archive {
      archive.insert_packet(packet).ok();
    }
  }

  pub fn refresh_soul_from_archive(&mut self, iuoc: IuocId) -> Result<bool, PersistError> {
    if let Some(archive) = &self.archive {
      if let Some(soul) = archive.load_soul(iuoc)? {
        self.souls.insert(soul.id, soul);
        Ok(true)
      } else {
        Ok(false)
      }
    } else {
      Ok(self.souls.contains_key(&iuoc))
    }
  }

  pub fn create_soul(&mut self) -> IuocId {
    let id = IuocId(ulid::Ulid::new().0);
    self.souls.insert(
      id,
      IUOC {
        id,
        quality: QualityScalar::INITIAL,
        incarnations: 0,
        bound_fwau: None,
        consent: ConsentGraph::default(),
        prefs: ReincarnationPrefs::default(),
      },
    );
    self.persist_soul(id);
    id
  }

  pub fn get(&self, id: IuocId) -> Option<&IUOC> {
    self.souls.get(&id)
  }

  pub fn get_mut(&mut self, id: IuocId) -> Option<&mut IUOC> {
    self.souls.get_mut(&id)
  }

  pub fn bind_fwau(
    &mut self,
    iuoc: IuocId,
    fwau_id: FwauId,
    avatar: EntityRef,
    bound_at: Tick,
  ) -> Result<FwauId, BindError> {
    let soul = self.souls.get_mut(&iuoc).ok_or(BindError::NotFound)?;

    if let Some(old) = soul.bound_fwau {
      if old != fwau_id {
        self.fwau_sessions.remove(&old);
      }
    }

    let fwau = FWAU {
      id: fwau_id,
      iuoc_id: iuoc,
      avatar_id: avatar,
      quality_snapshot: soul.quality,
      intent_buf: IntentQueueSnapshot { len: 0 },
      psi_budget: psi_budget(soul.quality),
      bound_at,
      memory_clearance: MemoryClearance::IncarnationOnly,
    };

    soul.bound_fwau = Some(fwau_id);
    soul.incarnations += 1;
    self.fwau_sessions.insert(fwau_id, fwau);
    self.persist_soul(iuoc);
    Ok(fwau_id)
  }

  /// Release a live FWAU without merging an experience packet (M11 shard cross).
  pub fn release_live_fwau(&mut self, fwau_id: FwauId) -> Option<IuocId> {
    let fwau = self.fwau_sessions.remove(&fwau_id)?;
    if let Some(soul) = self.souls.get_mut(&fwau.iuoc_id) {
      soul.bound_fwau = None;
      self.persist_soul(fwau.iuoc_id);
    }
    Some(fwau.iuoc_id)
  }

  pub fn merge_fwau(
    &mut self,
    fwau_id: FwauId,
    frame: FrameId,
    unbound_at: Tick,
    death_cause: &str,
    quality_after: QualityScalar,
  ) -> Option<ExperiencePacket> {
    let fwau = self.fwau_sessions.remove(&fwau_id)?;
    let soul = self.souls.get_mut(&fwau.iuoc_id)?;

    soul.bound_fwau = None;
    soul.quality = quality_after;

    let packet = ExperiencePacket {
      id: PacketId(ulid::Ulid::new().0),
      iuoc: fwau.iuoc_id,
      avatar: AvatarId(fwau.avatar_id.index as u64),
      frame,
      bound_at: fwau.bound_at,
      unbound_at,
      summary: format!("Life {} ended", soul.incarnations),
      death_cause: death_cause.to_string(),
    };

    self
      .packets
      .entry(fwau.iuoc_id)
      .or_default()
      .push(packet.clone());

    self.persist_soul(fwau.iuoc_id);
    self.persist_packet(&packet);

    Some(packet)
  }

  pub fn fwau(&self, id: FwauId) -> Option<&FWAU> {
    self.fwau_sessions.get(&id)
  }

  pub fn packets_for(&self, iuoc: IuocId) -> &[ExperiencePacket] {
    self.packets.get(&iuoc).map(|p| p.as_slice()).unwrap_or(&[])
  }
}

pub fn psi_budget(quality: QualityScalar) -> f32 {
  let s = quality.value();
  1.0 / (0.1 + s).max(0.01)
}

#[derive(Debug)]
pub enum BindError {
  NotFound,
  AlreadyBound,
}
