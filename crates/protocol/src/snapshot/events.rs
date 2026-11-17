//! Snapshot event segment (ARCHITECTURE §6.8.4). Only writes/reads a given list; redundancy
//! and ack policy live in the server (M0-019).
//!
//! `n:6` then per event `event_id:16 | type:5 | payload`. Position-like fields are raw
//! 14-bit codes whose metric meaning is defined by the consumer (M1), like `FireCmd.origin_hint`.
//!
//! | type | event        | payload bits |
//! |------|--------------|--------------|
//! | 1    | `Shot`       | shooter 8, weapon 8, muzzle 3x14, dir_yaw 10, dir_pitch 9 (PITCH_BITS_EVENT) |
//! | 2    | `Impact`     | pos 3x14, normal (octahedral) 12, material 8 |
//! | 3    | `HitConfirm` | target 8, zone 8, damage_q 8 |
//! | 4    | `DamageDir`  | angle 8, dmg_q 8 |
//! | 5    | `Explosion`  | pos 3x14, kind 8, radius_q 8 |
//! | 6    | `Suppress`   | source_dir 8, intensity 8 |
//! | 7    | `Marker`     | kind 8, pos 2x14 (x,z), ttl 8 |

use crate::bitio::{BitReader, BitWriter};
use crate::codec::CodecError;

pub const MAX_EVENTS: usize = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Shot {
        shooter: u8,
        weapon: u8,
        muzzle: [u16; 3],
        /// 10-bit yaw code.
        dir_yaw: u16,
        /// 9-bit pitch code.
        dir_pitch: u16,
    },
    Impact {
        pos: [u16; 3],
        /// 12-bit octahedral normal.
        normal: u16,
        material: u8,
    },
    HitConfirm {
        target: u8,
        zone: u8,
        damage_q: u8,
    },
    DamageDir {
        angle: u8,
        dmg_q: u8,
    },
    Explosion {
        pos: [u16; 3],
        kind: u8,
        radius_q: u8,
    },
    Suppress {
        source_dir: u8,
        intensity: u8,
    },
    /// Enemy marker; `pos` is quantised to 5 m and refreshed at most 1 Hz by the server (R3).
    Marker {
        kind: u8,
        pos: [u16; 2],
        ttl: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEntry {
    pub id: u16,
    pub event: Event,
}

impl EventEntry {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), CodecError> {
        w.write_bits(self.id as u64, 16)?;
        match self.event {
            Event::Shot {
                shooter,
                weapon,
                muzzle,
                dir_yaw,
                dir_pitch,
            } => {
                w.write_bits(1, 5)?;
                w.write_bits(shooter as u64, 8)?;
                w.write_bits(weapon as u64, 8)?;
                for m in muzzle {
                    w.write_bits(m as u64, 14)?;
                }
                w.write_bits(dir_yaw as u64, 10)?;
                w.write_bits(dir_pitch as u64, 9)
            }
            Event::Impact {
                pos,
                normal,
                material,
            } => {
                w.write_bits(2, 5)?;
                for p in pos {
                    w.write_bits(p as u64, 14)?;
                }
                w.write_bits(normal as u64, 12)?;
                w.write_bits(material as u64, 8)
            }
            Event::HitConfirm {
                target,
                zone,
                damage_q,
            } => {
                w.write_bits(3, 5)?;
                w.write_bits(target as u64, 8)?;
                w.write_bits(zone as u64, 8)?;
                w.write_bits(damage_q as u64, 8)
            }
            Event::DamageDir { angle, dmg_q } => {
                w.write_bits(4, 5)?;
                w.write_bits(angle as u64, 8)?;
                w.write_bits(dmg_q as u64, 8)
            }
            Event::Explosion {
                pos,
                kind,
                radius_q,
            } => {
                w.write_bits(5, 5)?;
                for p in pos {
                    w.write_bits(p as u64, 14)?;
                }
                w.write_bits(kind as u64, 8)?;
                w.write_bits(radius_q as u64, 8)
            }
            Event::Suppress {
                source_dir,
                intensity,
            } => {
                w.write_bits(6, 5)?;
                w.write_bits(source_dir as u64, 8)?;
                w.write_bits(intensity as u64, 8)
            }
            Event::Marker { kind, pos, ttl } => {
                w.write_bits(7, 5)?;
                w.write_bits(kind as u64, 8)?;
                w.write_bits(pos[0] as u64, 14)?;
                w.write_bits(pos[1] as u64, 14)?;
                w.write_bits(ttl as u64, 8)
            }
        }
    }

    pub fn decode(r: &mut BitReader<'_>) -> Result<Self, CodecError> {
        let id = r.read_bits(16)? as u16;
        let event = match r.read_bits(5)? {
            1 => Event::Shot {
                shooter: r.read_bits(8)? as u8,
                weapon: r.read_bits(8)? as u8,
                muzzle: [
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                ],
                dir_yaw: r.read_bits(10)? as u16,
                dir_pitch: r.read_bits(9)? as u16,
            },
            2 => Event::Impact {
                pos: [
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                ],
                normal: r.read_bits(12)? as u16,
                material: r.read_bits(8)? as u8,
            },
            3 => Event::HitConfirm {
                target: r.read_bits(8)? as u8,
                zone: r.read_bits(8)? as u8,
                damage_q: r.read_bits(8)? as u8,
            },
            4 => Event::DamageDir {
                angle: r.read_bits(8)? as u8,
                dmg_q: r.read_bits(8)? as u8,
            },
            5 => Event::Explosion {
                pos: [
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                    r.read_bits(14)? as u16,
                ],
                kind: r.read_bits(8)? as u8,
                radius_q: r.read_bits(8)? as u8,
            },
            6 => Event::Suppress {
                source_dir: r.read_bits(8)? as u8,
                intensity: r.read_bits(8)? as u8,
            },
            7 => Event::Marker {
                kind: r.read_bits(8)? as u8,
                pos: [r.read_bits(14)? as u16, r.read_bits(14)? as u16],
                ttl: r.read_bits(8)? as u8,
            },
            _ => return Err(CodecError::InvalidTag),
        };
        Ok(EventEntry { id, event })
    }
}
