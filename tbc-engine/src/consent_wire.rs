//! M18 — consent stamps on wire intents (spec §3 assist).

use crate::aum::AumCore;
use crate::intent::ConsentStamp;
use crate::types::IuocId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire-safe consent stamp (target IUOC as string in JSON).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireConsentStamp {
    #[serde(with = "crate::wire_json::compat")]
    pub target: u128,
    pub scope: String,
    pub expires_tick: u64,
}

impl WireConsentStamp {
    pub fn to_intent_stamp(&self) -> ConsentStamp {
        ConsentStamp {
            target: self.target,
            scope: self.scope.clone(),
            expires_tick: self.expires_tick,
        }
    }

    pub fn from_grant(target: IuocId, helper: IuocId, scope: &str, expires_tick: u64) -> Self {
        Self {
            target: target.0,
            scope: scope.into(),
            expires_tick,
        }
    }
}

pub fn consent_from_payload(payload: &Value) -> Option<ConsentStamp> {
    payload
        .get("consent")
        .and_then(|c| serde_json::from_value::<WireConsentStamp>(c.clone()).ok())
        .map(|w| w.to_intent_stamp())
}

pub fn verify_consent_stamp(
    aum: &AumCore,
    helper: IuocId,
    stamp: &ConsentStamp,
    now_tick: u64,
) -> bool {
    aum.iuoc.verify_consent_stamp(helper, stamp, now_tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Vec3;

    #[test]
    fn wire_stamp_verifies_after_grant() {
        let genesis = *blake3::hash(b"m18-wire").as_bytes();
        let mut aum = AumCore::boot_cluster(genesis);
        let target = aum.iuoc.create_soul();
        let helper = aum.iuoc.create_soul();
        let expires = aum.frames[0].now().0 + 50_000;
        aum.grant_consent(target, helper, "assist", 50_000)
            .expect("grant");
        let stamp =
            WireConsentStamp::from_grant(target, helper, "assist", expires).to_intent_stamp();
        let now = aum.frames[0].now().0;
        assert!(verify_consent_stamp(&aum, helper, &stamp, now));
    }

    #[test]
    fn expired_stamp_rejected() {
        let genesis = *blake3::hash(b"m18-expired").as_bytes();
        let mut aum = AumCore::boot_cluster(genesis);
        let target = aum.iuoc.create_soul();
        let helper = aum.iuoc.create_soul();
        let stamp = ConsentStamp {
            target: target.0,
            scope: "assist".into(),
            expires_tick: 1,
        };
        assert!(!verify_consent_stamp(&aum, helper, &stamp, 9999));
    }
}
