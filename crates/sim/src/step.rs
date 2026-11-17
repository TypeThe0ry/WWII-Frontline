//! Output of one simulation step. Variants are appended by later tasks.

use glam::Vec3;

use crate::components::Team;
use crate::ids::{EntityId, PlayerId};
use crate::weapon::HitZone;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SimEvent {
    PlayerSpawned {
        player: PlayerId,
        entity: EntityId,
    },
    PlayerRemoved {
        player: PlayerId,
        entity: EntityId,
    },
    /// A projectile left a barrel (ARCHITECTURE 6.8.4 `Shot`); `weapon` indexes `GameData::weapons`.
    Shot {
        shooter: EntityId,
        weapon: u16,
        muzzle: Vec3,
        /// Unit direction including recoil and spread.
        dir: Vec3,
    },
    /// Trigger pulled on an empty magazine.
    DryFire {
        shooter: EntityId,
    },
    /// A projectile hit the terrain (ARCHITECTURE 6.8.4 `Impact`).
    Impact {
        pos: Vec3,
        normal: Vec3,
        material: u8,
    },
    /// A projectile hit a player; `damage_hp` has been subtracted from the target's health.
    Hit {
        shooter: EntityId,
        target: EntityId,
        zone: HitZone,
        damage_hp: f32,
        dist_m: f32,
    },
    /// A player died (M0-013). `killer` is `None` when the killer has left the match;
    /// `weapon` indexes `GameData::weapons`; `headshot` = lethal hit in the head zone.
    Kill {
        victim: PlayerId,
        killer: Option<PlayerId>,
        weapon: u8,
        headshot: bool,
        dist_m: f32,
    },
    /// A player was suppressed (M1-017); sent to that player only, at most 10 Hz. `source_dir` is
    /// the world bearing to the source (256 steps, 0 = +z, clockwise to +x), `intensity` is
    /// `S x 2.55`.
    Suppress {
        target: EntityId,
        source_dir: u8,
        intensity: u8,
    },
    /// `suppressor` suppressed `victim` in the last 5 s and a teammate killed them (M1-017;
    /// scoring is M2).
    SuppressionAssist {
        suppressor: EntityId,
        victim: PlayerId,
    },
    /// A player went down (M1-016); `Hit` / `Kill` events cover the damage and the KIA.
    Downed {
        victim: PlayerId,
        killer: Option<PlayerId>,
        weapon: u8,
        headshot: bool,
    },
    /// Who gets what for a down or a KIA (GAMEDESIGN 7.5); `team_kill` = the killer was a
    /// teammate (no kill awarded).
    KillCredit {
        victim: PlayerId,
        killer: Option<PlayerId>,
        assists: Vec<PlayerId>,
        kind: crate::medical::CreditKind,
        team_kill: bool,
    },
    /// A medic got a teammate back on their feet.
    Revive {
        medic: EntityId,
        target: EntityId,
        target_player: PlayerId,
    },
    /// A grenade left a hand (M1-015).
    Thrown {
        thrower: EntityId,
        pos: Vec3,
        smoke: bool,
    },
    /// A blast went off at `pos` (`kind` 0 = grenade).
    Explosion {
        pos: Vec3,
        radius_m: f32,
        kind: u8,
    },
    SmokeStarted {
        pos: Vec3,
        radius_m: f32,
    },
    /// `count` smoke clouds ran out this tick.
    SmokeEnded {
        count: u8,
    },
    /// A melee strike landed (M1-013b); `damage_hp` has been subtracted, `backstab` = rear kill.
    MeleeHit {
        attacker: EntityId,
        target: EntityId,
        damage_hp: f32,
        backstab: bool,
    },
    /// A dead player re-entered the world at `pos` (M0-013).
    Spawn {
        player: PlayerId,
        pos: Vec3,
    },
}

/// Objective / resource / phase changes (M0-013+; empty for now).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleChange {
    /// A strongpoint changed hands the tick `|p|` reached 100 (M1-018). `repeat` = it changed
    /// hands again within 90 s (anti-farm, scoring is M2).
    ObjectiveOwner {
        objective: u16,
        owner: Team,
        reason: crate::rules::CaptureReason,
        repeat: bool,
    },
    /// The match entered a phase (`ends_at` = tick it ends, 0 = open ended).
    MatchPhase {
        phase: crate::rules::MatchPhaseKind,
        ends_at: u32,
    },
    MatchEnded {
        reason: crate::rules::VictoryReason,
        winner: Option<Team>,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepOutput {
    pub events: Vec<SimEvent>,
    pub rule_changes: Vec<RuleChange>,
}
