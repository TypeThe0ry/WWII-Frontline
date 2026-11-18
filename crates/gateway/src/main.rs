//! `frontline-gateway`: axum server over the library.

use std::net::SocketAddr;

use ed25519_dalek::SigningKey;
use frontline_gateway::app::{router, AppState};
use frontline_gateway::clock::Clock;
use frontline_gateway::config::Config;
use rand::RngCore;

fn load_key(path: Option<&str>) -> Result<SigningKey, String> {
    let Some(p) = path else {
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        tracing::warn!("no TICKET_SIGNING_KEY_FILE: using an ephemeral ticket key");
        return Ok(SigningKey::from_bytes(&seed));
    };
    match std::fs::read(p) {
        Ok(b) => {
            let seed: [u8; 32] = b.try_into().map_err(|_| "key file must be 32 raw bytes")?;
            Ok(SigningKey::from_bytes(&seed))
        }
        Err(_) => {
            let mut seed = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut seed);
            std::fs::write(p, seed).map_err(|e| e.to_string())?;
            Ok(SigningKey::from_bytes(&seed))
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("gateway: {e}");
            std::process::exit(2);
        }
    };
    let key = match load_key(cfg.ticket_key_file.as_deref()) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("gateway: {e}");
            std::process::exit(2);
        }
    };
    let bind = cfg.bind.clone();
    let app = router(AppState::new(cfg, Clock::real(), key))
        .into_make_service_with_connect_info::<SocketAddr>();
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind");
    tracing::info!(addr = %bind, "gateway listening");
    axum::serve(listener, app).await.expect("serve");
}
