//! M14 — CRDT or-set props for NPMR frames (ruleset `crdt.props`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtProp {
    pub id: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub author_fwau: u128,
    pub placed_tick: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PropStore {
    props: HashMap<String, CrdtProp>,
}

impl PropStore {
    pub fn new() -> Self {
        Self {
            props: HashMap::new(),
        }
    }

    /// Or-set add — rejects duplicate ids.
    pub fn add(&mut self, prop: CrdtProp) -> bool {
        if self.props.contains_key(&prop.id) {
            return false;
        }
        self.props.insert(prop.id.clone(), prop);
        true
    }

    pub fn all(&self) -> Vec<CrdtProp> {
        let mut out: Vec<CrdtProp> = self.props.values().cloned().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn len(&self) -> usize {
        self.props.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_set_rejects_duplicate() {
        let mut store = PropStore::new();
        let p = CrdtProp {
            id: "prop:a".into(),
            kind: "thought".into(),
            x: 1.0,
            y: 2.0,
            z: 0.0,
            author_fwau: 1,
            placed_tick: 10,
        };
        assert!(store.add(p.clone()));
        assert!(!store.add(p));
        assert_eq!(store.len(), 1);
    }
}
