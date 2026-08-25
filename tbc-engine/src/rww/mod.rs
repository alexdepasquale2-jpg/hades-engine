use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RwwMessage {
  pub id: String,
  pub subject: String,
  pub payload: Vec<u8>,
  pub at_tick: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RwwStatus {
  pub backend: String,
  pub nats_url: Option<String>,
  pub stream: Option<String>,
  pub connected: bool,
}

#[derive(Clone, Debug)]
pub struct RwwConfig {
  pub nats_url: Option<String>,
  pub stream_name: String,
}

impl RwwConfig {
  pub fn from_env() -> Self {
    let nats_url = std::env::var("TBC_NATS_URL").ok().filter(|s| !s.is_empty());
    let stream_name = std::env::var("TBC_NATS_STREAM").unwrap_or_else(|_| "TBC_RWW".into());
    Self {
      nats_url,
      stream_name,
    }
  }
}

mod memory;
mod nats;

pub use memory::MemoryStore;

/// RWW fabric — in-memory cache + optional NATS JetStream replication (M10).
#[derive(Clone)]
pub struct RwwBus {
  store: Arc<MemoryStore>,
  nats: Option<nats::NatsBridge>,
  status: RwwStatus,
}

impl RwwBus {
  pub fn new() -> Self {
    Self::open(RwwConfig {
      nats_url: None,
      stream_name: "TBC_RWW".into(),
    })
  }

  pub fn open_from_env() -> Self {
    Self::open(RwwConfig::from_env())
  }

  /// Share the in-memory store across processes in tests or single-host multi-node dev.
  pub fn with_store(config: RwwConfig, store: Arc<MemoryStore>) -> Self {
    if let Some(url) = config.nats_url.clone() {
      match nats::NatsBridge::connect(&url, &config.stream_name, store.clone()) {
        Ok(bridge) => {
          return Self {
            store,
            nats: Some(bridge),
            status: RwwStatus {
              backend: "nats".into(),
              nats_url: Some(url),
              stream: Some(config.stream_name),
              connected: true,
            },
          };
        }
        Err(e) => {
          tracing::warn!("NATS RWW unavailable ({}); using in-memory only", e);
        }
      }
    }

    Self {
      store,
      nats: None,
      status: RwwStatus {
        backend: "memory".into(),
        nats_url: config.nats_url,
        stream: None,
        connected: false,
      },
    }
  }

  pub fn open(config: RwwConfig) -> Self {
    let store = Arc::new(MemoryStore::new());
    Self::with_store(config, store)
  }

  pub fn status(&self) -> RwwStatus {
    self.status.clone()
  }

  pub fn publish(&self, subject: &str, payload: &[u8], at_tick: u64, durable: bool) {
    let msg = RwwMessage {
      id: ulid::Ulid::new().to_string(),
      subject: subject.to_string(),
      payload: payload.to_vec(),
      at_tick,
    };
    self.store.insert(msg.clone(), durable);
    if let Some(nats) = &self.nats {
      nats.publish(msg, durable);
    }
  }

  pub fn recent(&self, subject: &str, limit: usize) -> Vec<RwwMessage> {
    self.store.recent(subject, limit)
  }

  pub fn publish_fwau_bound(&self, fwau: u128, frame: &str, at_tick: u64) {
    let payload = serde_json::json!({
      "fwau": fwau.to_string(),
      "frame": frame,
      "event": "FwauBound"
    });
    self.publish(
      &format!("rww.bound.{}.{}", frame, fwau),
      payload.to_string().as_bytes(),
      at_tick,
      true,
    );
  }

  pub fn publish_handoff(&self, fwau: u128, from: &str, to: &str, at_tick: u64) {
    let payload = serde_json::json!({
      "fwau": fwau.to_string(),
      "from": from,
      "to": to,
      "event": "Handoff"
    });
    self.publish("rww.handoff", payload.to_string().as_bytes(), at_tick, true);
  }

  /// M11: spatial PMR shard crossing (multi-node).
  pub fn publish_shard_cross(&self, event: &crate::shard::ShardCrossEvent) {
    let payload = serde_json::to_vec(event).unwrap_or_default();
    self.publish("rww.shard.cross", &payload, event.tick, true);
  }
}

impl Default for RwwBus {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn publish_and_read() {
    let bus = RwwBus::new();
    bus.publish("rww.test", b"hello", 1, true);
    let msgs = bus.recent("rww.test", 10);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].payload, b"hello");
  }

  #[test]
  fn opens_memory_without_nats() {
    let bus = RwwBus::open(RwwConfig {
      nats_url: None,
      stream_name: "TBC_RWW".into(),
    });
    assert_eq!(bus.status().backend, "memory");
    assert!(!bus.status().connected);
  }
}
