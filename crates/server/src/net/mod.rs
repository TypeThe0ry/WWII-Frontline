//! Networking: transport seam and the WebSocket front end (M0-016). WebTransport (M0-017)
//! plugs in through [`transport::ClientTransport`] and reuses [`conn::run_connection`].

pub mod conn;
pub mod handshake;
pub mod limits;
pub mod transport;
pub mod ws;

/// A network front end that feeds the sim thread through [`crate::sim_thread::SimThreadHandle`].
pub trait Transport: Send {
    /// Short name for logs (`"ws"`, `"wt"`).
    fn name(&self) -> &'static str;
}

/// Placeholder used until the real transports land.
#[derive(Debug, Default)]
pub struct NullTransport;

impl Transport for NullTransport {
    fn name(&self) -> &'static str {
        "null"
    }
}
