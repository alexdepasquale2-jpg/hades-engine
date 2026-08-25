use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory RWW fabric (spec §9). Production uses NATS JetStream.
#[derive(Clone, Default)]
pub struct RwwBus {
  inner: Arc<RwLock<RwwInner>>,
}

#[derive(Default)]
struct RwwInner {
  durable: HashMap<String, Vec<RwwMessage>>,
  ephemeral: HashMap<String, Vec<RwwMessage>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RwwMessage {
  pub id: String,
  pub subject: String,
  pub payload: Vec<u8>,
  pub at_tick: u64,
}

impl RwwBus {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn publish(&self, subject: &str, payload: &[u8], at_tick: u64, durable: bool) {
    let msg = RwwMessage {
      id: ulid::Ulid::new().to_string(),
      subject: subject.to_string(),
      payload: payload.to_vec(),
      at_tick,
    };
    let mut g = self.inner.write().unwrap();
    let map = if durable {
      &mut g.durable
    } else {
      &mut g.ephemeral
    };
    map.entry(subject.to_string()).or_default().push(msg);
  }

  pub fn recent(&self, subject: &str, limit: usize) -> Vec<RwwMessage> {
    let g = self.inner.read().unwrap();
    let mut msgs: Vec<RwwMessage> = Vec::new();
    if let Some(d) = g.durable.get(subject) {
      msgs.extend(d.iter().cloned());
    }
    if let Some(e) = g.ephemeral.get(subject) {
      msgs.extend(e.iter().cloned());
    }
    msgs.sort_by_key(|m| m.at_tick);
    if msgs.len() > limit {
      msgs = msgs.split_off(msgs.len() - limit);
    }
    msgs
  }

  pub fn publish_fwau_bound(&self, fwau: u128, frame: &str, at_tick: u64) {
    let payload = serde_json::json!({
      "fwau": fwau,
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
      "fwau": fwau,
      "from": from,
      "to": to,
      "event": "Handoff"
    });
    self.publish("rww.handoff", payload.to_string().as_bytes(), at_tick, true);
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
}
