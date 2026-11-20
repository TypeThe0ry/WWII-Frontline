//! Output of one simulation step. Variants are appended by later tasks.

use glam::Vec3;

use crate::components::Team;
use crate::ids::{EntityId, PlayerId};
use crate::weapon::HitZone;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SimEvent {
    /// A hull hit an obstacle; `damage` is the hull damage dealt (M1-022).
    VehicleCollision {
        vehicle: EntityId,
        speed_mps: f32,
        damage: f32,
    },
    /// A vehicle ran a player over (the damage is already applied).
    Crush {
        vehicle: EntityId,
        victim: PlayerId,
        speed_kph: f32,
    },
    VehicleOverturned {
        vehicle: EntityId,
    },
    /// A player took a seat, moved seats, or left (`seat: None`).
    SeatChanged {
        player: PlayerId,
        vehicle: EntityId,
        seat: Option<u8>,
    },
    /// A shell struck a vehicle: armour zone index and outcome (0 pen, 1 no pen, 2 ricochet).
    ArmorHit {
        vehicle: EntityId,
        zone: u8,
        result: u8,
    },
    /// A vehicle module (0..6) or the hull (0xFF) took damage; `hp` is what is left.
    VehicleDamaged {
        vehicle: EntityId,
        module: u8,
        hp: f32,
    },
    /// The main gun fired shell type `shell` (0 AP, 1 HE, 2 WP).
    VehicleFired {
        vehicle: EntityId,
        shell: u8,
    },
    /// A Panzerfaust was launched.
    RocketFired {
        shooter: EntityId,
    },
    VehicleKnockedOut {
        vehicle: EntityId,
    },
    VehicleAmmoExplosion {
        vehicle: EntityId,
    },
    CrewKilled {
        player: PlayerId,
        vehicle: EntityId,
    },
    /// Roster, squad or role changed (M1-021).
    Roster(crate::squad::RosterDelta),
    /// An enemy entered a HQ protection zone: it takes damage after the warning (M1-019).
    HqWarning {
        player: PlayerId,
    },
    /// A Garrison finished building or was destroyed (M1-019b).
    SpawnPointState {
        id: u16,
        team: Team,
        pos: glam::Vec3,
        state: crate::spawn::garrison::GarrisonState,
    },
    StructureDestroyed {
        id: EntityId,
    },
    MuzzleFlash(crate::emplacement::at_gun::MuzzleFlash),
    OutpostChanged {
        point: crate::spawn::outpost::OutpostState,
        status: crate::spawn::outpost::OutpostStatus,
    },
    /// A Supply Crate was put on the ground (M1-020).
    CratePlaced {
        id: u32,
        team: Team,
        pos: glam::Vec3,
    },
    CrateDestroyed {
        id: u32,
    },
    /// A player refilled from an Ammo Crate (M1-020b).
    Resupplied {
        player: PlayerId,
        crate_id: u32,
        points: u32,
    },
    /// A message for one player ("cannot place here" etc.); `code` per `supply::system`.
    SupplyNotice {
        player: PlayerId,
        code: u8,
    },
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
        dist_m: f32,
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
    ScoreAction {
        player: PlayerId,
        action: crate::score::Action,
    },
    ScoreDelta(crate::score::ScoreDelta),
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
    pub server_actions: Vec<crate::serverrules::ServerAction>,
}

/// Wall-time samples in nanoseconds. Movement player timings are nested in `movement`.
/// `other` is the legacy total outside the selected system bodies. `schedule_other` is
/// only the part of that total inside the ECS schedule.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StepTimings {
    pub movement: u64,
    pub movement_players: u64,
    pub movement_slowest_player: u64,
    pub movement_slowest_entity: Option<EntityId>,
    pub weapons: u64,
    pub projectiles: u64,
    pub vehicles: u64,
    pub rockets: u64,
    pub shells: u64,
    pub throwables: u64,
    pub other: u64,
    pub prepare: u64,
    pub schedule_other: u64,
    pub outpost_rules: u64,
    pub facility_rules: u64,
    pub lagcomp: u64,
    pub reap_wrecks: u64,
    pub sync_squads: u64,
    pub refresh_views: u64,
    pub collect_output: u64,
}

/// Diagnostic resource installed only by a host that supplies a monotonic clock.
#[doc(hidden)]
#[derive(bevy_ecs::prelude::Resource)]
pub struct StepProfiler {
    pub(crate) clock_ns: fn() -> u64,
    pub(crate) timings: StepTimings,
}

impl StepProfiler {
    pub(crate) fn measure<'a>(
        profiler: &'a mut Option<bevy_ecs::prelude::ResMut<Self>>,
        select: fn(&mut StepTimings) -> &mut u64,
    ) -> Option<StepSample<'a>> {
        let profiler = profiler.as_deref_mut()?;
        Some(StepSample {
            clock_ns: profiler.clock_ns,
            start: (profiler.clock_ns)(),
            elapsed: select(&mut profiler.timings),
        })
    }
}

pub(crate) struct StepSample<'a> {
    clock_ns: fn() -> u64,
    start: u64,
    elapsed: &'a mut u64,
}

impl Drop for StepSample<'_> {
    fn drop(&mut self) {
        *self.elapsed = (self.clock_ns)().saturating_sub(self.start);
    }
}
