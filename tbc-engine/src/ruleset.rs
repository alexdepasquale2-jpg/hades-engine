use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ruleset {
  pub id: String,
  pub title: String,
  pub tightness: f32,
  pub dt_ms: u16,
  pub clock: ClockRatio,
  pub motion: MotionEnvelope,
  pub conservation: Conservation,
  pub death: DeathPolicy,
  pub psi: PsiPolicy,
  pub sleep: SleepPolicy,
  pub handoff: HandoffPolicy,
  pub crdt: Option<CrdtPolicy>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ClockRatio {
  pub parent: Option<String>,
  pub n: u32,
  pub k: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MotionEnvelope {
  pub gravity: f32,
  pub max_speed: f32,
  pub air_control: f32,
  pub blink: bool,
  pub c_info_m_s: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conservation {
  pub items: String,
  pub currency: Option<String>,
  pub clone_tax_entropy: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeathPolicy {
  pub unbind: bool,
  pub park_s: u32,
  pub rewind_s: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PsiPolicy {
  pub enabled: bool,
  pub base_cost: f32,
  pub scopes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SleepPolicy {
  pub delay_s: f32,
  pub kinematic_wake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandoffPolicy {
  pub allowed_targets: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrdtPolicy {
  pub props: String,
  pub presence: Option<String>,
}

impl Ruleset {
  pub fn pmr_prime() -> Self {
    Self {
      id: "pmr.v1".into(),
      title: "PMR-Prime".into(),
      tightness: 0.92,
      dt_ms: 50,
      clock: ClockRatio {
        parent: None,
        n: 1,
        k: 1,
      },
      motion: MotionEnvelope {
        gravity: 9.8,
        max_speed: 7.0,
        air_control: 0.35,
        blink: false,
        c_info_m_s: Some(300.0),
      },
      conservation: Conservation {
        items: "unique".into(),
        currency: Some("shard-local".into()),
        clone_tax_entropy: None,
      },
      death: DeathPolicy {
        unbind: true,
        park_s: 0,
        rewind_s: 0,
      },
      psi: PsiPolicy {
        enabled: true,
        base_cost: 0.04,
        scopes: vec!["PastOwn".into(), "FutureSelf".into()],
      },
      sleep: SleepPolicy {
        delay_s: 2.0,
        kinematic_wake: true,
      },
      handoff: HandoffPolicy {
        allowed_targets: vec!["npmr.academy.v1".into()],
      },
      crdt: None,
    }
  }

  pub fn npmr_academy() -> Self {
    Self {
      id: "npmr.academy.v1".into(),
      title: "NPMR-Academy".into(),
      tightness: 0.35,
      dt_ms: 200,
      clock: ClockRatio {
        parent: None,
        n: 1,
        k: 1,
      },
      motion: MotionEnvelope {
        gravity: 0.0,
        max_speed: 15.0,
        air_control: 1.0,
        blink: true,
        c_info_m_s: None,
      },
      conservation: Conservation {
        items: "clone-tax".into(),
        currency: None,
        clone_tax_entropy: Some(0.002),
      },
      death: DeathPolicy {
        unbind: false,
        park_s: 0,
        rewind_s: 5,
      },
      psi: PsiPolicy {
        enabled: true,
        base_cost: 0.005,
        scopes: vec![
          "PastOwn".into(),
          "PastShared".into(),
          "FutureIsland".into(),
          "RwwQuery".into(),
        ],
      },
      sleep: SleepPolicy {
        delay_s: 1.0,
        kinematic_wake: true,
      },
      handoff: HandoffPolicy {
        allowed_targets: vec!["pmr.v1".into(), "npmr.dream.v1".into()],
      },
      crdt: Some(CrdtPolicy {
        props: "or-set".into(),
        presence: Some("lww-register".into()),
      }),
    }
  }

  pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(s)
  }
}
