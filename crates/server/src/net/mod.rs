//! Transport seam. M0-016 (WebSocket) and M0-017 (WebTransport) implement [`Transport`];
//! M0-015 only fixes the trait and a no-op implementation.

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
