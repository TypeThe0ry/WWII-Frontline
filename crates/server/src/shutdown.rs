//! Graceful shutdown: SIGINT/SIGTERM and `--duration` expiry all end in the same flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cheap flag polled by the sim thread once per tick.
#[derive(Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Resolves when SIGINT or SIGTERM arrives; returns the signal name.
pub async fn wait_for_signal() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};
    let (Ok(mut term), Ok(mut int)) = (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) else {
        return std::future::pending().await;
    };
    tokio::select! {
        _ = term.recv() => "SIGTERM",
        _ = int.recv() => "SIGINT",
    }
}
