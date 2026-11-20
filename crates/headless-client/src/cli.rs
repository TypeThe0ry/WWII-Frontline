//! Command line (M0-027 behaviour 1).

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::transport::Kind;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TransportArg {
    Ws,
    Wt,
}

impl From<TransportArg> for Kind {
    fn from(t: TransportArg) -> Kind {
        match t {
            TransportArg::Ws => Kind::Ws,
            TransportArg::Wt => Kind::Wt,
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "headless-client",
    version,
    about = "Frontline headless client: N synthetic players over real WebSocket / WebTransport"
)]
pub struct Args {
    /// `host:port` of the transport to use (the server's --ws-port for ws, --wt-port for wt).
    #[arg(long, default_value = "127.0.0.1:7000")]
    pub url: String,
    #[arg(long, value_enum, default_value = "ws")]
    pub transport: TransportArg,
    /// Number of simultaneous connections (one tokio task each).
    #[arg(long, default_value_t = 10)]
    pub count: u32,
    /// Seconds each client stays connected after its handshake.
    #[arg(long, default_value_t = 60.0)]
    pub duration: f64,
    /// Seed of the synthetic input (client `i` uses a stream derived from `seed` and `i`).
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    /// Write the JSON report here.
    #[arg(long)]
    pub report: Option<PathBuf>,
    /// SHA-256 of the server certificate (hex or base64); required for `--transport wt`.
    #[arg(long)]
    pub cert_hash: Option<String>,
    /// Seconds at the start of each client's run that are left out of the bandwidth percentiles.
    #[arg(long, default_value_t = 5)]
    pub warmup: u32,
    /// Delay between starting consecutive clients, milliseconds.
    #[arg(long, default_value_t = 50)]
    pub ramp_ms: u64,
    /// Directory holding `maps/<map>/` (default: searched upwards from the cwd / executable).
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
    #[arg(long, default_value = "m0")]
    pub map: String,
    /// Request an unassigned rifleman deployment at HQ after Welcome (tactical maps).
    #[arg(long, conflicts_with_all = ["hash_only", "drive_vehicle"])]
    pub deploy_hq: bool,
    /// Do not give every client its own 127.x.y.z source address (the server allows only 4
    /// connections per IP, so loopback runs of more than 4 clients need the spread).
    #[arg(long)]
    pub no_bind_spread: bool,
    /// Client 0 boards a nearby friendly M4A1, drives, then holds the exit button before disconnecting.
    #[arg(long, conflicts_with = "hash_only")]
    pub drive_vehicle: bool,
    /// Run a legal-input control, with 3000 deg/s mouse flicks on even-numbered clients.
    #[arg(long, conflicts_with_all = ["hash_only", "drive_vehicle"])]
    pub anticheat_control: bool,
    /// Exercise mixed anticheat input fixtures on a local server.
    #[cfg(feature = "anticheat-test")]
    #[arg(long, conflicts_with_all = ["hash_only", "drive_vehicle", "anticheat_control"])]
    pub anticheat_mix: bool,
    /// Print the kinematic input-sequence hash of each client for this many ticks and exit
    /// (determinism check: equal for equal `--seed`).
    #[arg(long, value_name = "TICKS")]
    pub hash_only: Option<u32>,
    /// tokio worker threads (default: min(4, cores)).
    #[arg(long)]
    pub workers: Option<usize>,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        #[cfg(feature = "anticheat-test")]
        if self.anticheat_mix {
            let literal_loopback = self
                .url
                .parse::<std::net::SocketAddr>()
                .is_ok_and(|addr| addr.ip().is_loopback());
            if !literal_loopback || self.count < 6 || self.duration < 5.0 {
                return Err("--anticheat-mix needs a literal loopback address, six clients and at least five seconds".into());
            }
        }
        if self.count == 0 || self.count > 120 {
            return Err("--count must be 1..=120 (PlayerId space is 128)".into());
        }
        if !(self.duration.is_finite() && self.duration > 0.0) {
            return Err("--duration must be positive".into());
        }
        if self.drive_vehicle && self.duration < 5.0 {
            return Err("--drive-vehicle needs at least 5 seconds to board, drive and exit".into());
        }
        if self.transport == TransportArg::Wt && self.cert_hash.is_none() {
            return Err("--transport wt needs --cert-hash (see tools/dev/gen-dev-cert.sh)".into());
        }
        Ok(())
    }

    /// Whether the target host is on loopback (where binding distinct sources is possible).
    pub fn host_is_loopback(&self) -> bool {
        let host = self
            .url
            .rsplit_once(':')
            .map_or(self.url.as_str(), |(h, _)| h);
        host == "localhost" || host.starts_with("127.")
    }
}

/// Source address of client `i` on loopback: `127.<2 + i/65536>.<i/256 % 256>.<i % 256 + 1>`.
pub fn spread_ip(i: u32) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(
        127,
        (2 + (i >> 16)) as u8,
        ((i >> 8) & 255) as u8,
        ((i & 255) + 1) as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(a: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("headless-client").chain(a.iter().copied())).unwrap()
    }

    #[cfg(feature = "anticheat-test")]
    #[test]
    fn anticheat_mix_restricts_injection_to_literal_loopback() {
        assert!(parse(&["--anticheat-mix"]).validate().is_ok());
        assert!(parse(&["--anticheat-mix", "--url", "127.example.com:7000"])
            .validate()
            .is_err());
        assert!(parse(&["--anticheat-mix", "--count", "5"])
            .validate()
            .is_err());
    }

    #[test]
    fn defaults_and_task_cli_shape() {
        let a = parse(&[
            "--url",
            "127.0.0.1:7001",
            "--transport",
            "wt",
            "--count",
            "10",
            "--duration",
            "1800",
            "--seed",
            "1",
            "--report",
            "out.json",
            "--cert-hash",
            "ab",
        ]);
        assert_eq!(a.count, 10);
        assert_eq!(a.transport, TransportArg::Wt);
        assert!(a.validate().is_ok());
        assert!(a.host_is_loopback());
    }

    #[test]
    fn validation() {
        assert!(parse(&["--transport", "wt"]).validate().is_err());
        assert!(parse(&["--count", "0"]).validate().is_err());
        assert!(parse(&["--count", "500"]).validate().is_err());
        assert!(parse(&["--duration", "0"]).validate().is_err());
        assert!(!parse(&["--url", "example.com:7000"]).host_is_loopback());
    }

    #[test]
    fn spread_ips_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..1000 {
            assert!(seen.insert(spread_ip(i)));
        }
        assert_eq!(spread_ip(0).to_string(), "127.2.0.1");
    }
}
