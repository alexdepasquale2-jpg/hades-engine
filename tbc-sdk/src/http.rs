//! HTTP client for the tbc-server debug API.

use crate::error::SdkError;
use crate::types::{ConsentResponse, LoginResponse, StatusResponse};
use tbc_engine::assist::AssistResult;
use tbc_engine::frame::FrameSnapshot;
use tbc_engine::gameplay::{AttackResult, InteractResult};
use tbc_engine::psi::PsiResponse;
use tbc_engine::social::SpeakResult;
use tbc_engine::wire_json::id_json;

#[derive(Clone)]
pub struct TbcHttpClient {
    base: String,
    client: reqwest::Client,
}

impl TbcHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base = base_url.into().trim_end_matches('/').to_string();
        Self {
            base,
            client: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub async fn status(&self) -> Result<StatusResponse, SdkError> {
        self.get("/api/status").await
    }

    pub async fn login(&self) -> Result<LoginResponse, SdkError> {
        self.get("/api/login").await
    }

    pub async fn resume(&self, iuoc: u128) -> Result<LoginResponse, SdkError> {
        self.get(&format!("/api/resume?iuoc={}", iuoc)).await
    }

    pub async fn move_player(
        &self,
        fwau: u128,
        dx: f32,
        dy: f32,
        tick: Option<u64>,
    ) -> Result<serde_json::Value, SdkError> {
        self.post(
            "/api/move",
            serde_json::json!({
                "fwau": id_json(fwau),
                "dx": dx,
                "dy": dy,
                "tick": tick,
            }),
        )
        .await
    }

    pub async fn move_snapshot(
        &self,
        fwau: u128,
        dx: f32,
        dy: f32,
        tick: Option<u64>,
    ) -> Result<FrameSnapshot, SdkError> {
        let raw = self.move_player(fwau, dx, dy, tick).await?;
        serde_json::from_value(raw).map_err(SdkError::Json)
    }

    pub async fn blink(&self, fwau: u128, x: f32, y: f32) -> Result<serde_json::Value, SdkError> {
        self.post(
            "/api/blink",
            serde_json::json!({ "fwau": id_json(fwau), "x": x, "y": y }),
        )
        .await
    }

    pub async fn handoff(&self, fwau: u128, to_frame: &str) -> Result<LoginResponse, SdkError> {
        self.post(
            "/api/handoff",
            serde_json::json!({ "fwau": id_json(fwau), "to_frame": to_frame }),
        )
        .await
    }

    pub async fn attack(
        &self,
        fwau: u128,
        target_entity: Option<u32>,
    ) -> Result<AttackResult, SdkError> {
        self.post(
            "/api/attack",
            serde_json::json!({ "fwau": id_json(fwau), "target_entity": target_entity }),
        )
        .await
    }

    pub async fn interact(&self, fwau: u128) -> Result<InteractResult, SdkError> {
        self.post("/api/interact", serde_json::json!({ "fwau": id_json(fwau) }))
            .await
    }

    pub async fn assist(
        &self,
        fwau: u128,
        target_entity: Option<u32>,
    ) -> Result<AssistResult, SdkError> {
        self.post(
            "/api/assist",
            serde_json::json!({ "fwau": id_json(fwau), "target_entity": target_entity }),
        )
        .await
    }

    pub async fn speak(&self, fwau: u128, text: &str) -> Result<SpeakResult, SdkError> {
        self.post(
            "/api/speak",
            serde_json::json!({ "fwau": id_json(fwau), "text": text }),
        )
        .await
    }

    pub async fn psi(&self, fwau: u128, scope: &str) -> Result<PsiResponse, SdkError> {
        self.post(
            "/api/psi",
            serde_json::json!({ "fwau": id_json(fwau), "scope": scope }),
        )
        .await
    }

    pub async fn grant_consent(
        &self,
        fwau: u128,
        helper_iuoc: u128,
        scope: &str,
        ttl_ticks: u64,
    ) -> Result<ConsentResponse, SdkError> {
        self.post(
            "/api/consent",
            serde_json::json!({
                "fwau": id_json(fwau),
                "helper_iuoc": id_json(helper_iuoc),
                "scope": scope,
                "ttl_ticks": ttl_ticks,
            }),
        )
        .await
    }

    pub async fn health(&self) -> Result<serde_json::Value, SdkError> {
        self.get("/health").await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, SdkError> {
        let url = format!("{}{}", self.base, path);
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            return Err(SdkError::Http(format!("GET {} → {}", path, res.status())));
        }
        Ok(res.json().await?)
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, SdkError> {
        let url = format!("{}{}", self.base, path);
        let res = self.client.post(&url).json(&body).send().await?;
        if !res.status().is_success() {
            return Err(SdkError::Http(format!("POST {} → {}", path, res.status())));
        }
        Ok(res.json().await?)
    }
}
