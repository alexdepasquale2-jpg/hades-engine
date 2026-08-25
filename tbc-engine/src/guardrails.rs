use crate::types::FwauId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operational limits for a frame (M8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardrailConfig {
    pub step_budget_per_tick: usize,
    pub intent_rate_per_sec: u32,
    pub pending_intent_cap: usize,
    pub max_intents_per_tick: u32,
    pub max_rewinds_per_tick: u32,
    pub max_catchup: u32,
}

impl GuardrailConfig {
    pub fn for_dt_ms(dt_ms: u16) -> Self {
        let tick_hz = 1000.0 / dt_ms as f32;
        Self {
            step_budget_per_tick: 35_000,
            intent_rate_per_sec: (tick_hz * 2.0).ceil() as u32,
            pending_intent_cap: 64,
            max_intents_per_tick: 8,
            max_rewinds_per_tick: 4,
            max_catchup: 4,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuardrailReport {
    pub stall_count: u32,
    pub budget_overruns: u32,
    pub rate_limited: u32,
    pub queue_full: u32,
    pub rewinds_last_tick: u32,
    pub throttled_ticks: u32,
    pub within_budget: bool,
}

pub struct GuardrailState {
    pub config: GuardrailConfig,
    pub stall_count: u32,
    pub budget_overruns: u32,
    pub rate_limited: u32,
    pub queue_full: u32,
    pub throttled_ticks: u32,
    rewinds_this_tick: u32,
    intents_this_tick: HashMap<FwauId, u32>,
    rate_tokens: HashMap<FwauId, f32>,
}

impl GuardrailState {
    pub fn new(config: GuardrailConfig) -> Self {
        Self {
            config,
            stall_count: 0,
            budget_overruns: 0,
            rate_limited: 0,
            queue_full: 0,
            throttled_ticks: 0,
            rewinds_this_tick: 0,
            intents_this_tick: HashMap::new(),
            rate_tokens: HashMap::new(),
        }
    }

    pub fn begin_tick(&mut self, dt_ms: u16) {
        self.rewinds_this_tick = 0;
        self.intents_this_tick.clear();
        let refill = self.config.intent_rate_per_sec as f32 * dt_ms as f32 / 1000.0;
        for tokens in self.rate_tokens.values_mut() {
            *tokens = (*tokens + refill).min(self.config.intent_rate_per_sec as f32);
        }
    }

    pub fn record_stall(&mut self) {
        self.stall_count += 1;
    }

    pub fn record_budget_overrun(&mut self) {
        self.budget_overruns += 1;
    }

    pub fn record_throttled_tick(&mut self) {
        self.throttled_ticks += 1;
    }

    pub fn allow_rewind(&mut self) -> bool {
        if self.rewinds_this_tick >= self.config.max_rewinds_per_tick {
            return false;
        }
        self.rewinds_this_tick += 1;
        true
    }

    /// Per-tick + token-bucket intent admission.
    pub fn allow_intent(&mut self, fwau: FwauId) -> bool {
        let per_tick = self.intents_this_tick.entry(fwau).or_insert(0);
        if *per_tick >= self.config.max_intents_per_tick {
            self.rate_limited += 1;
            return false;
        }

        let tokens = self
            .rate_tokens
            .entry(fwau)
            .or_insert(self.config.intent_rate_per_sec as f32);
        if *tokens < 1.0 {
            self.rate_limited += 1;
            return false;
        }
        *tokens -= 1.0;
        *per_tick += 1;
        true
    }

    pub fn record_queue_full(&mut self) {
        self.queue_full += 1;
    }

    pub fn report(&self, within_budget: bool) -> GuardrailReport {
        GuardrailReport {
            stall_count: self.stall_count,
            budget_overruns: self.budget_overruns,
            rate_limited: self.rate_limited,
            queue_full: self.queue_full,
            rewinds_last_tick: self.rewinds_this_tick,
            throttled_ticks: self.throttled_ticks,
            within_budget,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_blocks_burst() {
        let mut g = GuardrailState::new(GuardrailConfig {
            step_budget_per_tick: 35_000,
            intent_rate_per_sec: 4,
            pending_intent_cap: 64,
            max_intents_per_tick: 100,
            max_rewinds_per_tick: 4,
            max_catchup: 4,
        });
        g.begin_tick(50);
        let fwau = FwauId(1);
        assert!(g.allow_intent(fwau));
        assert!(g.allow_intent(fwau));
        assert!(g.allow_intent(fwau));
        assert!(g.allow_intent(fwau));
        assert!(!g.allow_intent(fwau));
        assert_eq!(g.rate_limited, 1);
    }
}
