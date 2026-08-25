use super::RwwMessage;
use std::collections::HashMap;

pub struct MemoryStore {
  durable: RwLock<HashMap<String, Vec<RwwMessage>>>,
  ephemeral: RwLock<HashMap<String, Vec<RwwMessage>>>,
}

type RwLock<T> = std::sync::RwLock<T>;

impl MemoryStore {
  pub fn new() -> Self {
    Self {
      durable: RwLock::new(HashMap::new()),
      ephemeral: RwLock::new(HashMap::new()),
    }
  }

  pub fn insert(&self, msg: RwwMessage, durable: bool) {
    let map = if durable {
      &self.durable
    } else {
      &self.ephemeral
    };
    let mut g = map.write().unwrap();
    let bucket = g.entry(msg.subject.clone()).or_default();
    if bucket.iter().any(|m| m.id == msg.id) {
      return;
    }
    bucket.push(msg);
  }

  pub fn recent(&self, subject: &str, limit: usize) -> Vec<RwwMessage> {
    let mut msgs: Vec<RwwMessage> = Vec::new();
    if let Ok(g) = self.durable.read() {
      if let Some(d) = g.get(subject) {
        msgs.extend(d.iter().cloned());
      }
    }
    if let Ok(g) = self.ephemeral.read() {
      if let Some(e) = g.get(subject) {
        msgs.extend(e.iter().cloned());
      }
    }
    msgs.sort_by_key(|m| m.at_tick);
    if msgs.len() > limit {
      msgs = msgs.split_off(msgs.len() - limit);
    }
    msgs
  }
}
