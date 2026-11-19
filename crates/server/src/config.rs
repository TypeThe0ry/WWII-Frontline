//! CLI + `FRONTLINE_*` environment configuration (M0-015). Parsing and validation only: map
//! data is loaded by `world_loader` (M0-018), transports are bound by M0-016/017.

use clap::Parser;

pub const KNOWN_MAPS: &[&str] = &["m0", "carentan/m1"];
pub const KNOWN_MODES: &[&str] = &["sandbox", "frontline", "assault", "skirmish"];
pub const LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];
/// Players + bots upper bound (PlayerId 0..=127, ARCHITECTURE §8.1).
pub const MAX_BOTS: u16 = 128;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "frontline-server",
    version,
    about = "Frontline match server (one process = one match)"
)]
pub struct Config {
    /// Map id (only parsed/validated here).
    #[arg(long, env = "FRONTLINE_MAP", default_value = "m0")]
    pub map: String,
    /// Game mode.
    #[arg(long, env = "FRONTLINE_MODE", default_value = "sandbox")]
    pub mode: String,
    /// Match id (UUID); random when omitted.
    #[arg(long, env = "FRONTLINE_MATCH_ID")]
    pub match_id: Option<String>,
    /// Number of soldier bots to host (M0-026), split evenly between the teams; `PlayerId`s count
    /// down from 127 and are reserved against human joins.
    #[arg(long, env = "FRONTLINE_BOTS", default_value_t = 0)]
    pub bots: u16,
    /// Keep each team at this many players by adding/removing bots as humans come and go (M1-039c). 0 = off
    /// (static `--bots N`). Needs `--bots` > 0 so the bot host exists.
    #[arg(long, env = "FRONTLINE_BOT_FILL", default_value_t = 0)]
    pub bot_fill: u8,
    /// WebSocket port. 0 = ephemeral.
    #[arg(long, env = "FRONTLINE_WS_PORT", default_value_t = 7000)]
    pub ws_port: u16,
    /// WebTransport port (bound by M0-017). 0 = ephemeral.
    #[arg(long, env = "FRONTLINE_WT_PORT", default_value_t = 7001)]
    pub wt_port: u16,
    /// TLS certificate chain (PEM) for WebTransport. Needs `--key`.
    #[arg(long, env = "FRONTLINE_CERT")]
    pub cert: Option<std::path::PathBuf>,
    /// TLS private key (PEM) for WebTransport. Needs `--cert`.
    #[arg(long, env = "FRONTLINE_KEY")]
    pub key: Option<std::path::PathBuf>,
    /// Generate a throw-away self-signed certificate (13 days, ECDSA P-256) at startup and log its
    /// SHA-256 for `serverCertificateHashes`. Development only.
    #[arg(long, env = "FRONTLINE_DEV_CERT", default_value_t = false)]
    pub dev_cert: bool,
    /// Test scaffolding only: answer every valid C1 bundle with a C2 datagram **instead of**
    /// feeding it to the session layer (players then never move). Used by the transport stress
    /// tests that need a downlink before snapshots exist (M0-019). Implies `--empty-world`.
    #[arg(long, hide = true, env = "FRONTLINE_DEV_ECHO", default_value_t = false)]
    pub dev_echo: bool,
    /// Directory holding `maps/<map>/` (the repository's `data/`). Default: searched upwards
    /// from the working directory and the executable.
    #[arg(long, env = "FRONTLINE_DATA_DIR")]
    pub data_dir: Option<std::path::PathBuf>,
    /// Test scaffolding only: start with an empty collision world instead of loading the map
    /// (players cannot move). Never use in a real match.
    #[arg(
        long,
        hide = true,
        env = "FRONTLINE_EMPTY_WORLD",
        default_value_t = false
    )]
    pub empty_world: bool,
    /// Test fixture: spawn a driveable M4A1 beside the first player's M0 join point.
    #[arg(long, hide = true, conflicts_with_all = ["empty_world", "dev_echo", "gateway_url"])]
    pub dev_spawn_vehicle: bool,
    /// Prometheus `/metrics` port. 0 = ephemeral (actual port is logged).
    #[arg(long, env = "FRONTLINE_METRICS_PORT", default_value_t = 9100)]
    pub metrics_port: u16,
    /// Log level: error|warn|info|debug|trace.
    #[arg(long, env = "FRONTLINE_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
    /// Max log lines per second per call site.
    #[arg(long, env = "FRONTLINE_LOG_RATE_LIMIT", default_value_t = 50)]
    pub log_rate_limit: u32,
    /// Exit gracefully after this many seconds (load tests).
    #[arg(long, env = "FRONTLINE_DURATION")]
    pub duration: Option<f64>,
    /// Write a metrics summary JSON here on exit.
    #[arg(long, env = "FRONTLINE_REPORT")]
    pub report: Option<std::path::PathBuf>,
    /// Base URL of the gateway's internal API (`http://host:port`). Enables registration, the
    /// 1 s heartbeat and join-ticket verification (M1-029).
    #[arg(long, env = "GATEWAY_URL")]
    pub gateway_url: Option<String>,
    /// Bearer token of the gateway's internal API.
    #[arg(long, env = "INTERNAL_API_TOKEN", hide_env_values = true)]
    pub internal_api_token: Option<String>,
    /// Pin the gateway's ticket public key (base64url, 32 bytes): registration is refused when the
    /// gateway announces another key.
    #[arg(long, env = "TICKET_PUBLIC_KEY_FILE")]
    pub ticket_public_key_file: Option<std::path::PathBuf>,
    /// Run without a gateway: no registration and no join tickets. Development builds only.
    #[arg(long, env = "FRONTLINE_DEV_NO_GATEWAY", default_value_t = false)]
    pub dev_no_gateway: bool,
    /// Advertised address / region / name for the gateway's server list.
    #[arg(long, env = "FRONTLINE_PUBLIC_ADDR", default_value = "127.0.0.1")]
    pub public_addr: String,
    #[arg(long, env = "FRONTLINE_REGION", default_value = "local")]
    pub region: String,
    #[arg(long, env = "FRONTLINE_SERVER_NAME", default_value = "Frontline")]
    pub server_name: String,
    /// Where a result that could not be delivered is kept.
    #[arg(long, env = "FRONTLINE_SPOOL_DIR", default_value = "spool")]
    pub spool_dir: std::path::PathBuf,
    /// Pin the sim thread to this CPU core (deployment item M1-067; accepted, not applied).
    #[arg(long, env = "FRONTLINE_PIN_SIM_CORE")]
    pub pin_sim_core: Option<usize>,
}

fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    parts.len() == 5
        && parts
            .iter()
            .zip(lens)
            .all(|(p, l)| p.len() == l && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Pseudo-random v4-shaped UUID without a RNG dependency (match ids only need uniqueness).
fn random_uuid() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        chunk.copy_from_slice(&h.finish().to_le_bytes()[..chunk.len()]);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

impl Config {
    /// Validates and normalises (lower-cases enums, fills `match_id`).
    pub fn validate(mut self) -> Result<Config, String> {
        self.map = self.map.to_ascii_lowercase();
        self.mode = self.mode.to_ascii_lowercase();
        self.log_level = self.log_level.to_ascii_lowercase();
        if !KNOWN_MAPS.contains(&self.map.as_str()) {
            return Err(format!(
                "unknown --map '{}' (known: {})",
                self.map,
                KNOWN_MAPS.join(", ")
            ));
        }
        if !KNOWN_MODES.contains(&self.mode.as_str()) {
            return Err(format!(
                "unknown --mode '{}' (known: {})",
                self.mode,
                KNOWN_MODES.join(", ")
            ));
        }
        if self.mode != "sandbox" && self.map == "m0" {
            return Err(format!(
                "--mode {} is not playable on map m0 (m0 is the sandbox map)",
                self.mode
            ));
        }
        if !LOG_LEVELS.contains(&self.log_level.as_str()) {
            return Err(format!(
                "unknown --log-level '{}' (known: {})",
                self.log_level,
                LOG_LEVELS.join(", ")
            ));
        }
        if self.bots > MAX_BOTS {
            return Err(format!(
                "--bots {} exceeds the maximum of {MAX_BOTS}",
                self.bots
            ));
        }
        if self.log_rate_limit == 0 {
            return Err("--log-rate-limit must be >= 1".into());
        }
        let ports = [
            ("--ws-port", self.ws_port),
            ("--wt-port", self.wt_port),
            ("--metrics-port", self.metrics_port),
        ];
        for (i, (na, pa)) in ports.iter().enumerate() {
            for (nb, pb) in &ports[i + 1..] {
                if *pa != 0 && pa == pb {
                    return Err(format!("{na} and {nb} must differ (both {pa})"));
                }
            }
        }
        match (&self.cert, &self.key) {
            (Some(_), None) | (None, Some(_)) => {
                return Err("--cert and --key must be given together".into())
            }
            (Some(_), Some(_)) if self.dev_cert => {
                return Err("--dev-cert cannot be combined with --cert/--key".into())
            }
            _ => {}
        }
        if let Some(d) = self.duration {
            if !d.is_finite() || d <= 0.0 {
                return Err(format!(
                    "--duration must be a positive number of seconds (got {d})"
                ));
            }
        }
        if self.dev_no_gateway && self.gateway_url.is_some() {
            return Err("--dev-no-gateway cannot be combined with GATEWAY_URL".into());
        }
        if self.gateway_url.is_some() && self.internal_api_token.as_deref().unwrap_or("").is_empty()
        {
            return Err("GATEWAY_URL needs INTERNAL_API_TOKEN".into());
        }
        if !cfg!(debug_assertions) && self.dev_no_gateway {
            // Release builds only ever run without a gateway by omitting GATEWAY_URL (which logs a
            // loud warning); the explicit development switch does not exist there (PI-076).
            return Err("--dev-no-gateway is not available in release builds".into());
        }
        match &self.match_id {
            Some(id) if !is_uuid(id) => {
                return Err(format!(
                    "--match-id '{id}' is not a UUID (expected 8-4-4-4-12 hex)"
                ))
            }
            Some(id) => self.match_id = Some(id.to_ascii_lowercase()),
            None => self.match_id = Some(random_uuid()),
        }
        Ok(self)
    }

    /// WebTransport is served only when a certificate source was configured.
    pub fn wt_enabled(&self) -> bool {
        self.dev_cert || self.cert.is_some()
    }

    /// Numeric map id used by the simulation and `Welcome.map_id`.
    pub fn map_id(&self) -> u8 {
        u8::from(self.map == "carentan/m1")
    }

    /// Numeric mode id used in `Welcome.mode`.
    pub fn mode_id(&self) -> u8 {
        KNOWN_MODES
            .iter()
            .position(|m| *m == self.mode)
            .unwrap_or(0) as u8
    }

    /// Join tickets are enforced exactly when a gateway is configured.
    pub fn gateway_enabled(&self) -> bool {
        self.gateway_url.is_some()
    }

    pub fn match_id(&self) -> &str {
        self.match_id.as_deref().unwrap_or("")
    }

    /// Parses from an argv-style iterator (env vars still apply) and validates.
    pub fn try_parse_validated<I, T>(args: I) -> Result<Config, String>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Config::try_parse_from(args)
            .map_err(|e| e.to_string())
            .and_then(Config::validate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Config, String> {
        let mut v = vec!["frontline-server"];
        v.extend_from_slice(args);
        Config::try_parse_validated(v)
    }

    #[test]
    fn defaults_are_valid() {
        let c = parse(&[]).unwrap();
        assert_eq!((c.map.as_str(), c.mode.as_str()), ("m0", "sandbox"));
        assert!(is_uuid(c.match_id()));
    }

    #[test]
    fn carentan_m1_is_supported_by_the_match_cli() {
        let c = parse(&["--map", "Carentan/M1", "--mode", "assault"]).unwrap();
        assert_eq!(c.map, "carentan/m1");
        assert_eq!(c.map_id(), 1);
        assert_eq!(parse(&[]).unwrap().map_id(), 0);
    }

    #[test]
    fn rejects_illegal_combinations() {
        assert!(parse(&["--map", "nope"]).unwrap_err().contains("--map"));
        assert!(parse(&["--mode", "chess"]).unwrap_err().contains("--mode"));
        assert!(parse(&["--mode", "frontline"])
            .unwrap_err()
            .contains("not playable"));
        assert!(parse(&["--log-level", "loud"])
            .unwrap_err()
            .contains("--log-level"));
        assert!(parse(&["--bots", "500"]).unwrap_err().contains("--bots"));
        assert!(parse(&["--ws-port", "9100"])
            .unwrap_err()
            .contains("must differ"));
        assert!(parse(&["--duration", "0"])
            .unwrap_err()
            .contains("--duration"));
        assert!(parse(&["--match-id", "abc"]).unwrap_err().contains("UUID"));
        assert!(parse(&["--log-rate-limit", "0"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
    }

    #[test]
    fn gateway_options_are_cross_checked() {
        assert!(parse(&["--gateway-url", "http://g:1"])
            .unwrap_err()
            .contains("INTERNAL_API_TOKEN"));
        let ok = parse(&["--gateway-url", "http://g:1", "--internal-api-token", "t"]).unwrap();
        assert!(ok.gateway_enabled());
        assert!(parse(&[
            "--dev-no-gateway",
            "--gateway-url",
            "http://g:1",
            "--internal-api-token",
            "t"
        ])
        .is_err());
        // Debug builds (these tests) accept --dev-no-gateway; release builds refuse it.
        assert_eq!(parse(&["--dev-no-gateway"]).is_ok(), cfg!(debug_assertions));
    }

    #[test]
    fn accepts_explicit_values() {
        let c = parse(&[
            "--match-id",
            "123E4567-e89b-42d3-a456-426614174000",
            "--bots",
            "8",
            "--metrics-port",
            "0",
            "--ws-port",
            "0",
            "--wt-port",
            "0",
            "--duration",
            "2.5",
        ])
        .unwrap();
        assert_eq!(c.match_id(), "123e4567-e89b-42d3-a456-426614174000");
        assert_eq!(c.bots, 8);
        assert_eq!(c.duration, Some(2.5));
    }
}
