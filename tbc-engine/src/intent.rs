use crate::types::{FwauId, Tick};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verb {
    Move,
    Blink,
    Look,
    Interact,
    Attack,
    Speak,
    PsiQuery,
    Consent,
    Emote,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentStamp {
    pub target: u128,
    pub scope: String,
    pub expires_tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Intent {
    pub fwau: FwauId,
    pub tick: Tick,
    pub seq: u32,
    pub verb: Verb,
    pub payload: Vec<u8>,
    pub consent: Option<ConsentStamp>,
    pub checksum: u32,
}

#[derive(Clone, Debug, Default)]
pub struct IntentQueue {
    queue: Vec<Intent>,
    cap: usize,
}

impl IntentQueue {
    pub fn with_cap(cap: usize) -> Self {
        Self {
            queue: Vec::new(),
            cap,
        }
    }

    pub fn push(&mut self, intent: Intent) -> bool {
        if self.queue.len() >= self.cap {
            return false;
        }
        self.queue.push(intent);
        true
    }

    pub fn drain_for_tick(&mut self, tick: Tick) -> Vec<Intent> {
        let mut result = Vec::new();
        self.queue.retain(|i| {
            if i.tick <= tick {
                result.push(i.clone());
                false
            } else {
                true
            }
        });
        result
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
