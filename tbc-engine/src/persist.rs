//! M9 — durable IUOC souls and experience packet archive (SQLite).

use crate::iuoc::{ExperiencePacket, IUOC};
use crate::types::{FwauId, IuocId, PacketId};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS souls (
  id TEXT PRIMARY KEY NOT NULL,
  quality REAL NOT NULL,
  incarnations INTEGER NOT NULL,
  bound_fwau TEXT,
  consent_json TEXT NOT NULL,
  prefs_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS experience_packets (
  id TEXT PRIMARY KEY NOT NULL,
  iuoc_id TEXT NOT NULL,
  avatar INTEGER NOT NULL,
  frame_id INTEGER NOT NULL,
  bound_at INTEGER NOT NULL,
  unbound_at INTEGER NOT NULL,
  summary TEXT NOT NULL,
  death_cause TEXT NOT NULL,
  FOREIGN KEY (iuoc_id) REFERENCES souls(id)
);

CREATE INDEX IF NOT EXISTS idx_packets_iuoc ON experience_packets(iuoc_id);
";

#[derive(Debug, Error)]
pub enum PersistError {
  #[error("io: {0}")]
  Io(std::io::Error),
  #[error("sqlite: {0}")]
  Sqlite(rusqlite::Error),
  #[error("json: {0}")]
  Json(serde_json::Error),
  #[error("parse id: {0}")]
  ParseId(String),
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ArchiveStats {
  pub souls: u64,
  pub packets: u64,
  pub path: String,
}

pub struct SoulArchive {
  conn: Connection,
  path: String,
}

impl SoulArchive {
  pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistError> {
    let path_ref = path.as_ref();
    if let Some(parent) = path_ref.parent() {
      if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent).map_err(PersistError::Io)?;
      }
    }
    let conn = Connection::open(path_ref).map_err(PersistError::Sqlite)?;
    conn
      .execute_batch(SCHEMA)
      .map_err(PersistError::Sqlite)?;
    Ok(Self {
      conn,
      path: path_ref.to_string_lossy().into_owned(),
    })
  }

  pub fn path(&self) -> &str {
    &self.path
  }

  pub fn stats(&self) -> Result<ArchiveStats, PersistError> {
    let souls: u64 = self
      .conn
      .query_row("SELECT COUNT(*) FROM souls", [], |r| r.get(0))
      .map_err(PersistError::Sqlite)?;
    let packets: u64 = self
      .conn
      .query_row("SELECT COUNT(*) FROM experience_packets", [], |r| r.get(0))
      .map_err(PersistError::Sqlite)?;
    Ok(ArchiveStats {
      souls,
      packets,
      path: self.path.clone(),
    })
  }

  pub fn load_souls(&self) -> Result<Vec<IUOC>, PersistError> {
    let mut stmt = self
      .conn
      .prepare(
        "SELECT id, quality, incarnations, bound_fwau, consent_json, prefs_json FROM souls",
      )
      .map_err(PersistError::Sqlite)?;

    let mut out = Vec::new();
    let mut rows = stmt.query([]).map_err(PersistError::Sqlite)?;
    while let Some(row) = rows.next().map_err(PersistError::Sqlite)? {
      let id_str: String = row.get(0).map_err(PersistError::Sqlite)?;
      let id = IuocId(parse_u128(&id_str)?);
      let quality = row.get::<_, f32>(1).map_err(PersistError::Sqlite)?;
      let incarnations: u32 = row.get(2).map_err(PersistError::Sqlite)?;
      let bound_raw: Option<String> = row.get(3).map_err(PersistError::Sqlite)?;
      let bound_fwau = bound_raw
        .map(|s| parse_u128(&s).map(FwauId))
        .transpose()?;
      let consent_json: String = row.get(4).map_err(PersistError::Sqlite)?;
      let prefs_json: String = row.get(5).map_err(PersistError::Sqlite)?;
      out.push(IUOC {
        id,
        quality: crate::types::QualityScalar::clamped(quality),
        incarnations,
        bound_fwau,
        consent: serde_json::from_str(&consent_json).unwrap_or_default(),
        prefs: serde_json::from_str(&prefs_json).unwrap_or_default(),
      });
    }
    Ok(out)
  }

  pub fn load_soul(&self, iuoc: IuocId) -> Result<Option<IUOC>, PersistError> {
    let id_str = iuoc.0.to_string();
    let mut stmt = self
      .conn
      .prepare(
        "SELECT id, quality, incarnations, bound_fwau, consent_json, prefs_json
         FROM souls WHERE id = ?1",
      )
      .map_err(PersistError::Sqlite)?;
    let mut rows = stmt.query(params![id_str]).map_err(PersistError::Sqlite)?;
    if let Some(row) = rows.next().map_err(PersistError::Sqlite)? {
      let id_str: String = row.get(0).map_err(PersistError::Sqlite)?;
      let id = IuocId(parse_u128(&id_str)?);
      let quality = row.get::<_, f32>(1).map_err(PersistError::Sqlite)?;
      let incarnations: u32 = row.get(2).map_err(PersistError::Sqlite)?;
      let bound_raw: Option<String> = row.get(3).map_err(PersistError::Sqlite)?;
      let bound_fwau = bound_raw
        .map(|s| parse_u128(&s).map(FwauId))
        .transpose()?;
      let consent_json: String = row.get(4).map_err(PersistError::Sqlite)?;
      let prefs_json: String = row.get(5).map_err(PersistError::Sqlite)?;
      Ok(Some(IUOC {
        id,
        quality: crate::types::QualityScalar::clamped(quality),
        incarnations,
        bound_fwau,
        consent: serde_json::from_str(&consent_json).unwrap_or_default(),
        prefs: serde_json::from_str(&prefs_json).unwrap_or_default(),
      }))
    } else {
      Ok(None)
    }
  }

  pub fn load_packets(&self) -> Result<HashMap<IuocId, Vec<ExperiencePacket>>, PersistError> {
    let mut stmt = self
      .conn
      .prepare(
        "SELECT id, iuoc_id, avatar, frame_id, bound_at, unbound_at, summary, death_cause
         FROM experience_packets ORDER BY unbound_at ASC",
      )
      .map_err(PersistError::Sqlite)?;

    let mut map: HashMap<IuocId, Vec<ExperiencePacket>> = HashMap::new();
    let mut rows = stmt.query([]).map_err(PersistError::Sqlite)?;
    while let Some(row) = rows.next().map_err(PersistError::Sqlite)? {
      let id_str: String = row.get(0).map_err(PersistError::Sqlite)?;
      let iuoc_str: String = row.get(1).map_err(PersistError::Sqlite)?;
      let packet = ExperiencePacket {
        id: PacketId(parse_u128(&id_str)?),
        iuoc: IuocId(parse_u128(&iuoc_str)?),
        avatar: crate::types::AvatarId(row.get::<_, i64>(2).map_err(PersistError::Sqlite)? as u64),
        frame: crate::types::FrameId(row.get::<_, i32>(3).map_err(PersistError::Sqlite)? as u32),
        bound_at: crate::types::Tick(row.get::<_, i64>(4).map_err(PersistError::Sqlite)? as u64),
        unbound_at: crate::types::Tick(row.get::<_, i64>(5).map_err(PersistError::Sqlite)? as u64),
        summary: row.get(6).map_err(PersistError::Sqlite)?,
        death_cause: row.get(7).map_err(PersistError::Sqlite)?,
      };
      map.entry(packet.iuoc).or_default().push(packet);
    }
    Ok(map)
  }

  pub fn upsert_soul(&self, soul: &IUOC) -> Result<(), PersistError> {
    let consent_json = serde_json::to_string(&soul.consent).map_err(PersistError::Json)?;
    let prefs_json = serde_json::to_string(&soul.prefs).map_err(PersistError::Json)?;
    let bound = soul.bound_fwau.map(|f| f.0.to_string());
    self
      .conn
      .execute(
        "INSERT INTO souls (id, quality, incarnations, bound_fwau, consent_json, prefs_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
           quality = excluded.quality,
           incarnations = excluded.incarnations,
           bound_fwau = excluded.bound_fwau,
           consent_json = excluded.consent_json,
           prefs_json = excluded.prefs_json",
        params![
          soul.id.0.to_string(),
          soul.quality.value(),
          soul.incarnations,
          bound,
          consent_json,
          prefs_json,
        ],
      )
      .map_err(PersistError::Sqlite)?;
    Ok(())
  }

  pub fn insert_packet(&self, packet: &ExperiencePacket) -> Result<(), PersistError> {
    self
      .conn
      .execute(
        "INSERT OR REPLACE INTO experience_packets
         (id, iuoc_id, avatar, frame_id, bound_at, unbound_at, summary, death_cause)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
          packet.id.0.to_string(),
          packet.iuoc.0.to_string(),
          packet.avatar.0 as i64,
          packet.frame.0 as i32,
          packet.bound_at.0 as i64,
          packet.unbound_at.0 as i64,
          packet.summary,
          packet.death_cause,
        ],
      )
      .map_err(PersistError::Sqlite)?;
    Ok(())
  }
}

fn parse_u128(s: &str) -> Result<u128, PersistError> {
  s.parse::<u128>().map_err(|_| PersistError::ParseId(s.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::iuoc::IuocRegistry;
  use crate::types::{FrameId, QualityScalar, Tick};

  #[test]
  fn archive_roundtrip() {
    let dir = std::env::temp_dir().join(format!("tbc-archive-{}", ulid::Ulid::new()));
    let path = dir.join("archive.db");
    let archive = SoulArchive::open(&path).expect("open");

    let mut reg = IuocRegistry::new();
    let iuoc = reg.create_soul();
    reg
      .bind_fwau(
        iuoc,
        FwauId(42),
        crate::iuoc::EntityRef {
          index: 1,
          generation: 0,
        },
        Tick(10),
      )
      .unwrap();
    reg.merge_fwau(
      FwauId(42),
      FrameId(1),
      Tick(100),
      "test-death",
      QualityScalar::clamped(0.4),
    );

    archive.upsert_soul(reg.get(iuoc).unwrap()).unwrap();
    let packet = reg.packets_for(iuoc)[0].clone();
    archive.insert_packet(&packet).unwrap();

    let archive2 = SoulArchive::open(&path).expect("reopen");
    let souls = archive2.load_souls().unwrap();
    assert_eq!(souls.len(), 1);
    assert_eq!(souls[0].incarnations, 1);
    assert!(souls[0].bound_fwau.is_none());

    let packets = archive2.load_packets().unwrap();
    assert_eq!(packets.get(&iuoc).map(|p| p.len()), Some(1));

    std::fs::remove_dir_all(&dir).ok();
  }
}
