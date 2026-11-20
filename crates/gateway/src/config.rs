//! Environment configuration (`GATEWAY_BIND`, `TICKET_SIGNING_KEY_FILE`, `INTERNAL_API_TOKEN`,
//! `TOKEN_SECRET`, `ALLOWED_ORIGINS`, `DATABASE_URL`, `OAUTH_ENABLED`).

#[derive(Clone)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    /// Ed25519 seed (32 raw bytes) file; generated and written when missing.
    pub ticket_key_file: Option<String>,
    pub internal_token: String,
    /// HS256 secret of guest session tokens.
    pub token_secret: Vec<u8>,
    pub allowed_origins: Vec<String>,
    /// Expose the reserved OAuth routes. No provider integration is configured.
    pub oauth_enabled: bool,
    /// Read the client IP from `X-Forwarded-For` (behind a reverse proxy; also used by tests).
    pub trust_forwarded: bool,
    pub protocol_version: u16,
    pub ip_limit_per_min: u32,
    pub account_limit_per_min: u32,
    pub heartbeat_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Config, String> {
        let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let internal_token = get("INTERNAL_API_TOKEN").ok_or("INTERNAL_API_TOKEN is required")?;
        let secret = get("TOKEN_SECRET").ok_or("TOKEN_SECRET is required")?;
        Ok(Config {
            bind: get("GATEWAY_BIND").unwrap_or_else(|| "127.0.0.1:8080".into()),
            database_url: get("DATABASE_URL").unwrap_or_else(|| "sqlite://gateway.sqlite3".into()),
            ticket_key_file: get("TICKET_SIGNING_KEY_FILE"),
            internal_token,
            token_secret: secret.into_bytes(),
            allowed_origins: get("ALLOWED_ORIGINS")
                .map(|s| s.split(',').map(|x| x.trim().to_owned()).collect())
                .unwrap_or_default(),
            oauth_enabled: get("OAUTH_ENABLED").as_deref() == Some("1"),
            trust_forwarded: get("TRUST_FORWARDED").is_some(),
            ..Config::test()
        })
    }

    /// Defaults used by tests (fixed secrets, forwarded IPs trusted).
    pub fn test() -> Config {
        Config {
            bind: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:".into(),
            ticket_key_file: None,
            internal_token: "internal-test-token".into(),
            token_secret: b"test-secret-test-secret-test-secret".to_vec(),
            allowed_origins: vec!["https://play.example".into()],
            oauth_enabled: false,
            trust_forwarded: true,
            protocol_version: frontline_protocol::PROTOCOL_VERSION,
            ip_limit_per_min: 10,
            account_limit_per_min: 6,
            heartbeat_timeout_ms: 5_000,
        }
    }
}
