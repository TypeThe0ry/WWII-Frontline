//! Networking: transport seam, the WebSocket front end (M0-016) and the WebTransport front end
//! (M0-017). Both implement [`transport::ClientTransport`] and share [`conn::run_connection`].

pub mod conn;
pub mod datagram;
pub mod handshake;
pub mod limits;
pub mod transport;
pub mod wt;
pub mod wt_conn;
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
