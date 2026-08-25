use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("http: {0}")]
    Http(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
}
