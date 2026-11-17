//! Damage model and combat bookkeeping (M1-014, GAMEDESIGN 6.6 / 7.1 / 7.5).
//!
//! `damage` turns a hit into hit points lost and an outcome, `credit` keeps the recent-damage
//! window kill attribution needs (M1-016), `lagcomp_history` is the hitbox ring buffer that
//! lag compensation rewinds (M1-025). Hit zone volumes themselves live in
//! [`crate::weapon::hitbox`].

pub mod credit;
pub mod damage;
pub mod lagcomp_history;

pub use damage::{
    compute_damage, ff_mult, DamageKind, DamageOutcome, DamageResult, HitInfo, KillReason,
    TargetCtx,
};
