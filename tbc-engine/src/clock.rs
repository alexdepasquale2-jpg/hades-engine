use crate::types::Tick;
use std::time::Duration;

/// Fixed-timestep Δt clock with accumulator and bounded catch-up.
pub struct DeltaTClock {
    pub dt: Duration,
    pub tick: Tick,
    pub acc: Duration,
    pub max_catchup: u32,
}

impl DeltaTClock {
    pub fn new(dt_ms: u64) -> Self {
        Self {
            dt: Duration::from_millis(dt_ms),
            tick: Tick(0),
            acc: Duration::ZERO,
            max_catchup: 4,
        }
    }

    /// Drain elapsed wall time into simulation ticks. Returns number of steps to run.
    pub fn drain(&mut self, elapsed: Duration) -> u32 {
        self.acc += elapsed;
        let mut steps = 0u32;
        while self.acc >= self.dt && steps < self.max_catchup {
            self.tick = Tick(self.tick.0 + 1);
            self.acc -= self.dt;
            steps += 1;
        }
        if steps == self.max_catchup && self.acc >= self.dt {
            self.acc = Duration::ZERO;
        }
        steps
    }

    pub fn now(&self) -> Tick {
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_runs_ticks() {
        let mut clock = DeltaTClock::new(50);
        let steps = clock.drain(Duration::from_millis(100));
        assert_eq!(steps, 2);
        assert_eq!(clock.tick.0, 2);
    }

    #[test]
    fn catchup_caps_at_max() {
        let mut clock = DeltaTClock::new(50);
        let steps = clock.drain(Duration::from_millis(500));
        assert_eq!(steps, 4);
        assert_eq!(clock.acc, Duration::ZERO);
    }
}
