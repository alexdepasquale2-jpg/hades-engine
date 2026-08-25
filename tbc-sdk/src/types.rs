//! JSON types matching tbc-server HTTP responses.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub iuoc: u128,
    pub fwau: u128,
    pub quality_band: String,
    pub frame: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub tick: u64,
    pub dt_ms: u16,
    pub entities: usize,
    pub fwau_count: usize,
    pub ruleset: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentResponse {
    pub pact: String,
    pub granted: bool,
    pub has_consent: bool,
}
