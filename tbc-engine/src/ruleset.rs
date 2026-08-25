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
  #[serde(default)]
  pub verbs: VerbPolicies,
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

/// M13 — ruleset-driven verb policies loaded from JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerbPolicies {
  pub attack: AttackPolicy,
  pub interact: InteractPolicy,
  #[serde(default)]
  pub speak: SpeakPolicy,
}

impl Default for VerbPolicies {
  fn default() -> Self {
    Self {
      attack: AttackPolicy::default(),
      interact: InteractPolicy::default(),
      speak: SpeakPolicy::default(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeakPolicy {
  pub enabled: bool,
  pub range_m: f32,
}

impl Default for SpeakPolicy {
  fn default() -> Self {
    Self {
      enabled: true,
      range_m: 24.0,
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackPolicy {
  pub enabled: bool,
  pub damage: f32,
  pub range_m: f32,
  pub stamina_cost: f32,
  pub harm_entropy: f32,
}

impl Default for AttackPolicy {
  fn default() -> Self {
    Self {
      enabled: true,
      damage: 34.0,
      range_m: 8.0,
      stamina_cost: 12.0,
      harm_entropy: 0.5,
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractPolicy {
  pub enabled: bool,
  pub range_m: f32,
  pub item_prefix: String,
}

impl Default for InteractPolicy {
  fn default() -> Self {
    Self {
      enabled: true,
      range_m: 4.0,
      item_prefix: "echo".into(),
    }
  }
}

pub struct RulesetRegistry {
  rulesets: std::collections::HashMap<String, Ruleset>,
}

impl RulesetRegistry {
  pub fn empty() -> Self {
    Self {
      rulesets: std::collections::HashMap::new(),
    }
  }

  pub fn insert(&mut self, ruleset: Ruleset) {
    self.rulesets.insert(ruleset.id.clone(), ruleset);
  }

  pub fn get(&self, id: &str) -> Option<&Ruleset> {
    self.rulesets.get(id)
  }

  pub fn load_dir(path: &std::path::Path) -> Result<Self, RulesetLoadError> {
    let mut reg = Self::empty();
    if !path.is_dir() {
      return Err(RulesetLoadError::NotFound(path.display().to_string()));
    }
    for entry in std::fs::read_dir(path).map_err(RulesetLoadError::Io)? {
      let entry = entry.map_err(RulesetLoadError::Io)?;
      let p = entry.path();
      if p.extension().and_then(|e| e.to_str()) != Some("json") {
        continue;
      }
      let text = std::fs::read_to_string(&p).map_err(RulesetLoadError::Io)?;
      let ruleset = Ruleset::from_json(&text).map_err(|e| RulesetLoadError::Json(p.display().to_string(), e))?;
      reg.insert(ruleset);
    }
    if reg.rulesets.is_empty() {
      return Err(RulesetLoadError::NotFound(path.display().to_string()));
    }
    Ok(reg)
  }

  pub fn boot_defaults() -> Self {
    let path = std::env::var("TBC_RULESETS_PATH").unwrap_or_else(|_| {
      format!("{}/../rulesets", env!("CARGO_MANIFEST_DIR"))
    });
    Self::load_dir(std::path::Path::new(&path)).unwrap_or_else(|e| {
      tracing::warn!("ruleset dir load failed ({}); using embedded defaults", e);
      let mut reg = Self::empty();
      reg.insert(Ruleset::pmr_prime());
      reg.insert(Ruleset::npmr_academy());
      reg
    })
  }
}

#[derive(Debug, thiserror::Error)]
pub enum RulesetLoadError {
  #[error("io: {0}")]
  Io(std::io::Error),
  #[error("ruleset dir not found or empty: {0}")]
  NotFound(String),
  #[error("json {0}: {1}")]
  Json(String, serde_json::Error),
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
      verbs: VerbPolicies {
        attack: AttackPolicy {
          enabled: true,
          damage: 34.0,
          range_m: 8.0,
          stamina_cost: 12.0,
          harm_entropy: 0.5,
        },
        interact: InteractPolicy {
          enabled: true,
          range_m: 4.0,
          item_prefix: "echo".into(),
        },
        speak: SpeakPolicy {
          enabled: true,
          range_m: 20.0,
        },
      },
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
      verbs: VerbPolicies {
        attack: AttackPolicy {
          enabled: false,
          damage: 5.0,
          range_m: 6.0,
          stamina_cost: 5.0,
          harm_entropy: 0.1,
        },
        interact: InteractPolicy {
          enabled: true,
          range_m: 6.0,
          item_prefix: "thought".into(),
        },
        speak: SpeakPolicy {
          enabled: true,
          range_m: 48.0,
        },
      },
    }
  }

  pub fn npmr_dream() -> Self {
    Self {
      id: "npmr.dream.v1".into(),
      title: "NPMR-Dream".into(),
      tightness: 0.2,
      dt_ms: 200,
      clock: ClockRatio {
        parent: None,
        n: 1,
        k: 1,
      },
      motion: MotionEnvelope {
        gravity: 0.0,
        max_speed: 18.0,
        air_control: 1.0,
        blink: true,
        c_info_m_s: None,
      },
      conservation: Conservation {
        items: "clone-tax".into(),
        currency: None,
        clone_tax_entropy: Some(0.001),
      },
      death: DeathPolicy {
        unbind: false,
        park_s: 0,
        rewind_s: 8,
      },
      psi: PsiPolicy {
        enabled: true,
        base_cost: 0.003,
        scopes: vec![
          "PastOwn".into(),
          "PastShared".into(),
          "FutureIsland".into(),
          "RwwQuery".into(),
        ],
      },
      sleep: SleepPolicy {
        delay_s: 0.8,
        kinematic_wake: true,
      },
      handoff: HandoffPolicy {
        allowed_targets: vec!["pmr.v1".into(), "npmr.academy.v1".into()],
      },
      crdt: Some(CrdtPolicy {
        props: "or-set".into(),
        presence: Some("lww-register".into()),
      }),
      verbs: VerbPolicies {
        attack: AttackPolicy {
          enabled: false,
          damage: 3.0,
          range_m: 5.0,
          stamina_cost: 4.0,
          harm_entropy: 0.05,
        },
        interact: InteractPolicy {
          enabled: true,
          range_m: 8.0,
          item_prefix: "dream".into(),
        },
        speak: SpeakPolicy {
          enabled: true,
          range_m: 64.0,
        },
      },
    }
  }

  pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(s)
  }
}
