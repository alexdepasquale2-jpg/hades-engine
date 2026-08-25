use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WIRE_MAGIC: u32 = 0x54424301;

/// Length-prefixed JSON envelope for QUIC reliable streams (spec §10).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireMessage {
    pub kind: String,
    pub tick: Option<u64>,
    /// Stringified u128 — JSON numbers cannot hold full ULIDs.
    pub fwau: Option<String>,
    pub payload: Value,
}

impl WireMessage {
    pub fn login() -> Self {
        Self {
            kind: "login".into(),
            tick: None,
            fwau: None,
            payload: Value::Null,
        }
    }

    pub fn move_intent(fwau: u128, tick: u64, dx: f32, dy: f32) -> Self {
        Self {
            kind: "move".into(),
            tick: Some(tick),
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "dx": dx, "dy": dy }),
        }
    }

    pub fn blink(fwau: u128, x: f32, y: f32) -> Self {
        Self {
            kind: "blink".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "x": x, "y": y }),
        }
    }

    pub fn handoff(fwau: u128, to_frame: &str) -> Self {
        Self {
            kind: "handoff".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "to_frame": to_frame }),
        }
    }

    pub fn attack(fwau: u128, target_entity: Option<u32>) -> Self {
        Self {
            kind: "attack".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "target_entity": target_entity }),
        }
    }

    pub fn interact(fwau: u128) -> Self {
        Self {
            kind: "interact".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: Value::Null,
        }
    }

    pub fn assist(fwau: u128, target_entity: Option<u32>) -> Self {
        Self {
            kind: "assist".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "target_entity": target_entity }),
        }
    }

    pub fn speak(fwau: u128, text: &str) -> Self {
        Self {
            kind: "speak".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "text": text }),
        }
    }

    pub fn psi_query(fwau: u128, scope: &str) -> Self {
        Self {
            kind: "psi".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "scope": scope }),
        }
    }

    pub fn consent_grant(fwau: u128, helper_iuoc: u128, scope: &str, ttl_ticks: u64) -> Self {
        Self {
            kind: "consent".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({
                "helper_iuoc": helper_iuoc.to_string(),
                "scope": scope,
                "ttl_ticks": ttl_ticks,
            }),
        }
    }

    pub fn snapshot_request(fwau: u128) -> Self {
        Self {
            kind: "snapshot".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: Value::Null,
        }
    }

    pub fn result(kind: &str, fwau: u128, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload,
        }
    }

    pub fn snapshot(payload: Value) -> Self {
        Self {
            kind: "snapshot".into(),
            tick: None,
            fwau: None,
            payload,
        }
    }

    pub fn login_ok(fwau: u128, iuoc: u128, snapshot: Value) -> Self {
        Self {
            kind: "login_ok".into(),
            tick: None,
            fwau: Some(fwau.to_string()),
            payload: serde_json::json!({ "iuoc": iuoc.to_string(), "snapshot": snapshot }),
        }
    }

    pub fn fwau_u128(&self) -> Option<u128> {
        self.fwau.as_ref().and_then(|s| s.parse().ok())
    }
}

pub fn encode_reliable(msg: &WireMessage) -> Result<Vec<u8>, serde_json::Error> {
    let body = serde_json::to_vec(msg)?;
    let len = body.len() as u32;
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&WIRE_MAGIC.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn decode_reliable(buf: &[u8]) -> Result<WireMessage, String> {
    if buf.len() < 8 {
        return Err("too short".into());
    }
    let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    if magic != WIRE_MAGIC {
        return Err("bad magic".into());
    }
    let len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
    if buf.len() < 8 + len {
        return Err("incomplete".into());
    }
    serde_json::from_slice(&buf[8..8 + len]).map_err(|e| e.to_string())
}

/// Consume one or more framed messages from a growable buffer.
pub fn drain_reliable(buf: &mut Vec<u8>) -> Vec<WireMessage> {
    let mut out = Vec::new();
    loop {
        if buf.len() < 8 {
            break;
        }
        let len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
        let total = 8 + len;
        if buf.len() < total {
            break;
        }
        if let Ok(msg) = decode_reliable(&buf[..total]) {
            out.push(msg);
        }
        buf.drain(..total);
    }
    out
}

/// Unreliable datagram: Move/Look (≤256 B spec). Layout: magic(4) + fwau(16) + tick(8) + dx(4) + dy(4) = 36 B
pub fn encode_move_datagram(fwau: u128, tick: u64, dx: f32, dy: f32) -> Vec<u8> {
    let mut out = Vec::with_capacity(36);
    out.extend_from_slice(&WIRE_MAGIC.to_le_bytes());
    out.extend_from_slice(&fwau.to_le_bytes());
    out.extend_from_slice(&tick.to_le_bytes());
    out.extend_from_slice(&dx.to_le_bytes());
    out.extend_from_slice(&dy.to_le_bytes());
    out
}

pub fn decode_move_datagram(buf: &[u8]) -> Option<(u128, u64, f32, f32)> {
    if buf.len() < 36 {
        return None;
    }
    let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    if magic != WIRE_MAGIC {
        return None;
    }
    let fwau = u128::from_le_bytes(buf[4..20].try_into().unwrap());
    let tick = u64::from_le_bytes(buf[20..28].try_into().unwrap());
    let dx = f32::from_le_bytes(buf[28..32].try_into().unwrap());
    let dy = f32::from_le_bytes(buf[32..36].try_into().unwrap());
    Some((fwau, tick, dx, dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_reliable() {
        let msg = WireMessage::move_intent(42, 10, 1.0, 0.0);
        let enc = encode_reliable(&msg).unwrap();
        let dec = decode_reliable(&enc).unwrap();
        assert_eq!(dec.kind, "move");
        assert_eq!(dec.tick, Some(10));
        assert_eq!(dec.fwau_u128(), Some(42));
    }

    #[test]
    fn datagram_roundtrip() {
        let d = encode_move_datagram(99, 5, -1.0, 0.5);
        let (f, t, dx, _dy) = decode_move_datagram(&d).unwrap();
        assert_eq!(f, 99);
        assert_eq!(t, 5);
        assert!((dx + 1.0).abs() < 0.001);
    }
}
