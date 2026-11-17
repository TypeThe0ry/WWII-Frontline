//! Development / test echo: every valid C1 bundle is answered with a C2 datagram carrying the
//! seq of the newest command (u16 LE) and `ack_snapshot_tick` (u16 LE). Stands in for the real
//! session layer (M0-018) in loopback and stress tests (`--dev-echo`); never enabled by default.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use bytes::Bytes;
use crossbeam_channel::{Receiver, RecvTimeoutError};
use frontline_protocol::Channel;

use super::conn::{Inbound, Sessions};
use super::transport::Rel;

/// Payload echoed for `bundle` (4 bytes).
pub fn echo_payload(b: &Inbound) -> [u8; 4] {
    let seq = b.bundle.cmds.last().map(|c| c.seq).unwrap_or(0);
    let mut p = [0u8; 4];
    p[..2].copy_from_slice(&seq.to_le_bytes());
    p[2..].copy_from_slice(&b.bundle.ack_snapshot_tick.to_le_bytes());
    p
}

pub struct DevEcho {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl DevEcho {
    pub fn spawn(sessions: Arc<Sessions>, rx: Receiver<Inbound>) -> DevEcho {
        let stop = Arc::new(AtomicBool::new(false));
        let s = stop.clone();
        let join = std::thread::Builder::new()
            .name("dev-echo".into())
            .spawn(move || loop {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(i) => {
                        if let Some(h) = sessions.handle_of(i.player) {
                            h.send(
                                Channel::C2,
                                Bytes::copy_from_slice(&echo_payload(&i)),
                                Rel::Unreliable,
                            );
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if s.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            })
            .expect("spawn dev-echo");
        DevEcho {
            stop,
            join: Some(join),
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}
