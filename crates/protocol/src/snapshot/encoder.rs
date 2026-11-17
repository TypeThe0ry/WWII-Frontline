//! Server-side snapshot encoder: [`SnapshotEncoder`] (all clients) and [`SnapshotBuilder`]
//! (one packet). See the [`crate::snapshot`] module docs for the packet layout.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::bitio::BitWriter;
use crate::codec::CodecError;
use crate::snapshot::baseline::{ClientBaselines, MAX_SHORT_TICK_AGE};
use crate::snapshot::crc::crc16_ccitt;
use crate::snapshot::entity::{
    find_state, make_sent, write_delta, write_keyframe, write_remove, EntityState,
};
use crate::snapshot::events::{EventEntry, MAX_EVENTS};
use crate::snapshot::header::{
    header_bits, Header, FLAG_CRC, FLAG_FULL_TICK, FLAG_HAS_BASELINE, FLAG_HAS_SELF,
};
use crate::snapshot::self_state::SelfState;
use crate::snapshot::{MAX_ENTITIES, MAX_PACKET};

pub type ClientId = u32;
pub type Tick = u32;

/// Per-client baselines for every connected client.
#[derive(Debug, Default)]
pub struct SnapshotEncoder {
    clients: BTreeMap<ClientId, ClientBaselines>,
}

impl SnapshotEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a snapshot for `client` at `tick`. `budget` is the maximum packet size in bytes
    /// (clamped to 1200; see [`DEFAULT_BUDGET`]).
    pub fn begin(&mut self, client: ClientId, tick: Tick, budget: usize) -> SnapshotBuilder<'_> {
        self.clients.entry(client).or_default().begin(tick, budget)
    }

    /// The client acknowledged the snapshot of `ack_tick`: promote it to the new baseline.
    pub fn on_ack(&mut self, client: ClientId, ack_tick: Tick) {
        if let Some(c) = self.clients.get_mut(&client) {
            c.on_ack(ack_tick);
        }
    }

    pub fn remove_client(&mut self, client: ClientId) {
        self.clients.remove(&client);
    }

    pub fn client(&self, client: ClientId) -> Option<&ClientBaselines> {
        self.clients.get(&client)
    }

    /// Direct access for callers that want to encode different clients on different threads.
    pub fn client_mut(&mut self, client: ClientId) -> &mut ClientBaselines {
        self.clients.entry(client).or_default()
    }
}

impl ClientBaselines {
    /// Start a snapshot at `tick` for this client (see [`SnapshotEncoder::begin`]).
    pub fn begin(&mut self, tick: Tick, budget: usize) -> SnapshotBuilder<'_> {
        let full_tick = match self.last_acked {
            Some(a) => tick.wrapping_sub(a) > MAX_SHORT_TICK_AGE,
            None => true,
        };
        let baseline_tick = self.usable_baseline(tick).map(|b| b.tick);
        SnapshotBuilder {
            st: self,
            tick,
            budget_bits: budget.min(MAX_PACKET) * 8,
            crc: true,
            full_tick,
            baseline_tick,
            ack_input_seq: 0,
            input_slack: 0,
            self_w: None,
            ev_w: BitWriter::new(),
            ev_n: 0,
            ent_w: BitWriter::new(),
            ent_n: 0,
            scratch: BitWriter::new(),
            sent: Vec::new(),
            seen: [0; 4],
            any_delta: false,
            keyframes: 0,
            deltas: 0,
        }
    }
}

/// Builds one snapshot packet. Dropping it without [`finish`](Self::finish) leaves the
/// client's baseline state untouched.
pub struct SnapshotBuilder<'a> {
    st: &'a mut ClientBaselines,
    tick: Tick,
    budget_bits: usize,
    crc: bool,
    full_tick: bool,
    baseline_tick: Option<u32>,
    ack_input_seq: u16,
    input_slack: u8,
    self_w: Option<BitWriter>,
    ev_w: BitWriter,
    ev_n: usize,
    ent_w: BitWriter,
    ent_n: usize,
    scratch: BitWriter,
    sent: Vec<EntityState>,
    seen: [u64; 4],
    any_delta: bool,
    keyframes: usize,
    deltas: usize,
}

impl SnapshotBuilder<'_> {
    /// Enable/disable the CRC-16 trailer (disable on the WebSocket/TCP fallback). Default on.
    pub fn with_crc(mut self, crc: bool) -> Self {
        self.crc = crc;
        self
    }

    /// Header fields: last input `seq` processed for this client and the input-buffer slack
    /// hint (0..=15).
    pub fn set_ack(&mut self, ack_input_seq: u16, input_slack: u8) -> Result<(), CodecError> {
        if input_slack > 15 {
            return Err(CodecError::Overflow);
        }
        self.ack_input_seq = ack_input_seq;
        self.input_slack = input_slack;
        Ok(())
    }

    fn used_bits(&self) -> usize {
        let self_bits = self.self_w.as_ref().map_or(0, BitWriter::bit_len);
        header_bits(self.full_tick)
            + self_bits
            + 6
            + self.ev_w.bit_len()
            + 7
            + self.ent_w.bit_len()
            + if self.crc { 16 } else { 0 }
    }

    fn fits(&self, extra_bits: usize) -> bool {
        (self.used_bits() + extra_bits).div_ceil(8) * 8 <= self.budget_bits
    }

    /// Bits used so far (header and trailer included), rounded up to whole bytes at finish.
    pub fn bits_used(&self) -> usize {
        self.used_bits()
    }

    /// Bits used by entity records so far (excludes the 7-bit count).
    pub fn entity_bits(&self) -> usize {
        self.ent_w.bit_len()
    }

    /// `(keyframes, deltas)` records written so far.
    pub fn record_counts(&self) -> (usize, usize) {
        (self.keyframes, self.deltas)
    }

    /// Add the receiving player's own state. Errors if added twice or out of range; returns
    /// `Ok(false)` if it does not fit the budget.
    pub fn add_self(&mut self, s: &SelfState) -> Result<bool, CodecError> {
        if self.self_w.is_some() {
            return Err(CodecError::InvalidValue);
        }
        let mut w = BitWriter::new();
        s.encode(&mut w)?;
        if !self.fits(w.bit_len()) {
            return Ok(false);
        }
        self.self_w = Some(w);
        Ok(true)
    }

    /// Append events (stops at the first one that does not fit or at 63). Returns how many were
    /// written; never writes half an event.
    pub fn add_events(&mut self, events: &[EventEntry]) -> Result<usize, CodecError> {
        let mut n = 0;
        for e in events {
            if self.ev_n >= MAX_EVENTS {
                break;
            }
            self.scratch.clear();
            e.encode(&mut self.scratch)?;
            if !self.fits(self.scratch.bit_len()) {
                break;
            }
            self.ev_w.append(&self.scratch)?;
            self.ev_n += 1;
            n += 1;
        }
        Ok(n)
    }

    fn mark(&mut self, id: u8) -> Result<(), CodecError> {
        let (w, b) = (id as usize / 64, id as u64 % 64);
        if self.seen[w] >> b & 1 != 0 {
            return Err(CodecError::InvalidValue);
        }
        self.seen[w] |= 1 << b;
        Ok(())
    }

    fn is_seen(&self, id: u8) -> bool {
        self.seen[id as usize / 64] >> (id as u64 % 64) & 1 != 0
    }

    /// Add one entity. Delta against the baseline when the client has it, keyframe otherwise.
    /// `near`: full-precision rotation (yaw 10 bit + pitch); far: yaw 8 bit, pitch frozen at the
    /// baseline value. Returns `Ok(false)` (and writes nothing) when it does not fit or the
    /// 127-record limit is reached. A repeated `id` in one packet is an error.
    pub fn add_entity(&mut self, s: &EntityState, near: bool) -> Result<bool, CodecError> {
        s.validate()?;
        if self.is_seen(s.id) {
            return Err(CodecError::InvalidValue);
        }
        if self.ent_n >= MAX_ENTITIES {
            return Ok(false);
        }
        let base = self
            .baseline_tick
            .and_then(|_| self.st.baseline.as_ref())
            .and_then(|b| find_state(&b.states, s.id))
            .copied();
        self.scratch.clear();
        let sent = match &base {
            Some(b) => {
                let sent = make_sent(Some(b), s, near);
                write_delta(&mut self.scratch, b, &sent, near)?;
                sent
            }
            None => {
                write_keyframe(&mut self.scratch, s)?;
                *s
            }
        };
        if !self.fits(self.scratch.bit_len()) {
            return Ok(false);
        }
        self.mark(s.id)?;
        self.ent_w.append(&self.scratch)?;
        self.ent_n += 1;
        self.sent.push(sent);
        if base.is_some() {
            self.any_delta = true;
            self.deltas += 1;
        } else {
            self.keyframes += 1;
        }
        Ok(true)
    }

    /// Tell the client an entity left its view. `Ok(false)` if it does not fit.
    pub fn add_removed(&mut self, id: u8) -> Result<bool, CodecError> {
        if self.is_seen(id) {
            return Err(CodecError::InvalidValue);
        }
        if self.ent_n >= MAX_ENTITIES {
            return Ok(false);
        }
        self.scratch.clear();
        write_remove(&mut self.scratch, id)?;
        if !self.fits(self.scratch.bit_len()) {
            return Ok(false);
        }
        self.mark(id)?;
        self.ent_w.append(&self.scratch)?;
        self.ent_n += 1;
        Ok(true)
    }

    /// Serialise the packet and remember what was sent (for [`SnapshotEncoder::on_ack`]).
    pub fn finish(self) -> Vec<u8> {
        let mut flags = 0u8;
        if self.full_tick {
            flags |= FLAG_FULL_TICK;
        }
        if self.crc {
            flags |= FLAG_CRC;
        }
        if self.self_w.is_some() {
            flags |= FLAG_HAS_SELF;
        }
        let mut baseline_raw = 0;
        if let (true, Some(bt)) = (self.any_delta, self.baseline_tick) {
            flags |= FLAG_HAS_BASELINE;
            baseline_raw = (self.tick.wrapping_sub(bt) - 1) as u8;
        }
        let hdr = Header {
            flags,
            tick: self.tick,
            ack_input_seq: self.ack_input_seq,
            input_slack: self.input_slack,
            baseline_raw,
        };
        // Every value below was range-checked when it was added, so these writes cannot fail.
        let mut w = BitWriter::with_capacity(self.used_bits() / 8 + 2);
        let _ = hdr.write(&mut w);
        if let Some(s) = &self.self_w {
            let _ = w.append(s);
        }
        let _ = w.write_bits(self.ev_n as u64, 6);
        let _ = w.append(&self.ev_w);
        let _ = w.write_bits(self.ent_n as u64, 7);
        let _ = w.append(&self.ent_w);
        let mut bytes = w.finish();
        if self.crc {
            let c = crc16_ccitt(&bytes);
            bytes.extend_from_slice(&c.to_be_bytes());
        }
        self.st.push_sent(self.tick, self.sent);
        bytes
    }
}
