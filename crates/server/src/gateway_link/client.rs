//! HTTP client for the gateway's internal API (reqwest, plain HTTP on the private network).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::result::MatchResult;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize)]
pub struct RegisterReq {
    pub name: String,
    pub map: String,
    pub mode: String,
    pub region: String,
    pub tags: Vec<String>,
    pub version: String,
    pub protocol_version: u16,
    pub data_hash: u64,
    pub cap: u8,
    pub addr: String,
    pub ws_port: u16,
    pub wt_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterResp {
    pub match_id: String,
    pub ticket_public_key: String,
    #[serde(default)]
    pub ticket_key_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Heartbeat {
    pub humans: u8,
    pub bots: u8,
    pub phase: String,
    pub time_left_s: u32,
}

#[derive(Debug)]
pub enum LinkError {
    Http(String),
    Status(u16),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Http(e) => write!(f, "{e}"),
            LinkError::Status(s) => write!(f, "gateway answered HTTP {s}"),
        }
    }
}

impl std::error::Error for LinkError {}

#[derive(Clone)]
pub struct GatewayClient {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl GatewayClient {
    pub fn new(base_url: &str, token: &str) -> Result<Self, String> {
        let base = base_url.trim_end_matches('/').to_string();
        if !base.starts_with("http://") {
            return Err(
                "GATEWAY_URL must start with http:// (the internal link is plain HTTP on the \
                 private network; TLS is terminated by the reverse proxy, PI-076)"
                    .into(),
            );
        }
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(GatewayClient {
            http,
            base,
            token: token.to_string(),
        })
    }

    async fn post<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<reqwest::Response, LinkError> {
        let r = self
            .http
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .map_err(|e| LinkError::Http(e.without_url().to_string()))?;
        if r.status().is_success() {
            Ok(r)
        } else {
            Err(LinkError::Status(r.status().as_u16()))
        }
    }

    pub async fn register(&self, r: &RegisterReq) -> Result<RegisterResp, LinkError> {
        self.post("/internal/matches/register", r)
            .await?
            .json::<RegisterResp>()
            .await
            .map_err(|e| LinkError::Http(e.without_url().to_string()))
    }

    /// `attempts` tries with exponential backoff starting at `first_delay`.
    pub async fn register_with_retry(
        &self,
        r: &RegisterReq,
        attempts: u32,
        first_delay: Duration,
    ) -> Result<RegisterResp, LinkError> {
        let mut delay = first_delay;
        let mut last = LinkError::Http("no attempt made".into());
        for i in 0..attempts.max(1) {
            match self.register(r).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    tracing::warn!(
                        event = "gateway_register_failed",
                        attempt = u64::from(i + 1),
                        error = %e,
                        "gateway registration failed"
                    );
                    last = e;
                }
            }
            if i + 1 < attempts {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
        Err(last)
    }

    pub async fn heartbeat(&self, match_id: &str, h: &Heartbeat) -> Result<(), LinkError> {
        self.post(&format!("/internal/matches/{match_id}/heartbeat"), h)
            .await
            .map(|_| ())
    }

    pub async fn submit_result(&self, match_id: &str, r: &MatchResult) -> Result<(), LinkError> {
        self.post(&format!("/internal/matches/{match_id}/result"), r)
            .await
            .map(|_| ())
    }
}
