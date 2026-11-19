//! `D = dmg x falloff(d) x part_mult x pen_factor x ff_mult` and the state it leaves the target
//! in (GAMEDESIGN 6.6, 7.1). Damage producers apply HP changes; the deaths stage resolves outcomes.

use crate::data::defs::WeaponDef;
use crate::weapon::HitZone;

/// Hit points a player has right after going down or being revived (GAMEDESIGN 7.1).
pub const DOWNED_HP: f32 = 40.0;
/// A hit that leaves hit points at or below this kills outright (overkill, GAMEDESIGN 7.1).
pub const OVERKILL_HP: f32 = -60.0;
/// Head multiplier against a helmet for pistol and SMG rounds (GAMEDESIGN 6.6).
pub const HELMET_HEAD_MULT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    Bullet,
    Explosion,
    Melee,
    Vehicle,
    Tank,
    /// Falling (M1-016b); never friendly fire.
    Fall,
}

/// Friendly-fire coefficient per damage kind (GAMEDESIGN 6.6; 【待调参】). Self damage from your
/// own explosives is 1.0 and is the caller's concern (same attacker and victim).
pub fn ff_mult(kind: DamageKind) -> f32 {
    match kind {
        DamageKind::Bullet | DamageKind::Tank => 0.5,
        DamageKind::Explosion => 0.7,
        DamageKind::Melee => 0.0,
        DamageKind::Vehicle | DamageKind::Fall => 1.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitInfo {
    pub zone: HitZone,
    /// Distance the projectile flew (not the straight line), metres.
    pub travelled_m: f32,
    /// Penetration factor from M1-012 (1.0 when nothing was penetrated).
    pub pen_mult: f32,
    pub kind: DamageKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetCtx {
    pub hp: f32,
    pub same_team: bool,
    /// The target wears a helmet (all infantry in M1, PI-053).
    pub helmet: bool,
    /// Friendly fire is on (M1 default).
    pub friendly_fire: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillReason {
    Headshot,
    Overkill,
    Explosion,
    Vehicle,
    /// A producer marked this lethal hit as KIA (e.g. a backstab or a high fall).
    Forced,
    /// A hit depleted the target's downed HP without another outright-KIA rule.
    DownedDepleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
    /// HP remains positive; the target stays in its current state.
    Hit,
    Downed,
    Killed(KillReason),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageResult {
    pub hp_after: f32,
    pub outcome: DamageOutcome,
    pub damage: f32,
    /// Hit points below zero the hit went (0 when it did not kill or down).
    pub overkill: f32,
}

fn is_pistol_or_smg(def: &WeaponDef) -> bool {
    matches!(def.class.as_str(), "pistol" | "smg")
}

fn zone_mult(def: &WeaponDef, zone: HitZone, helmet: bool) -> f32 {
    let m = &def.gameplay.part_mult;
    let head = if helmet && is_pistol_or_smg(def) {
        // Never above the data value: weapons that already have a lower head multiplier keep it.
        (m.head as f32).min(HELMET_HEAD_MULT)
    } else {
        m.head as f32
    };
    match zone {
        HitZone::Head => head,
        HitZone::Neck => m.neck as f32,
        HitZone::Torso => m.torso as f32,
        // PI-007: a missing pelvis multiplier falls back to the torso one.
        HitZone::Pelvis => m.pelvis.unwrap_or(m.torso) as f32,
        HitZone::Limb => m.limb as f32,
    }
}

/// Hit points a hit takes off, before the outcome is decided.
pub fn damage_amount(def: &WeaponDef, h: &HitInfo, t: &TargetCtx) -> f32 {
    let mut d = def.gameplay.damage_torso_hp as f32
        * crate::weapon::ballistics::falloff(&def.gameplay.falloff, h.travelled_m)
        * zone_mult(def, h.zone, t.helmet)
        * h.pen_mult;
    if t.same_team {
        d *= if t.friendly_fire {
            ff_mult(h.kind)
        } else {
            0.0
        };
    }
    d
}

/// Previews one hit on a standing target, without a producer-specific KIA override.
pub fn compute_damage(def: &WeaponDef, h: &HitInfo, t: &TargetCtx) -> DamageResult {
    let damage = damage_amount(def, h, t);
    let hp_after = t.hp - damage;
    DamageResult {
        hp_after,
        outcome: classify_outcome(hp_after, h.kind, h.zone, false, false),
        damage,
        overkill: (-hp_after).max(0.0),
    }
}

/// Classifies already-applied damage without subtracting HP or changing life state.
pub(crate) fn classify_outcome(
    hp_after: f32,
    kind: DamageKind,
    zone: HitZone,
    downed: bool,
    direct_kia: bool,
) -> DamageOutcome {
    if hp_after > 0.0 {
        DamageOutcome::Hit
    } else if let Some(reason) = outright_kill_reason(hp_after, kind, zone, direct_kia) {
        DamageOutcome::Killed(reason)
    } else if downed {
        DamageOutcome::Killed(KillReason::DownedDepleted)
    } else {
        DamageOutcome::Downed
    }
}

/// Outright KIA rules for a hit that would otherwise down a standing target.
pub(crate) fn outright_kill_reason(
    hp_after: f32,
    kind: DamageKind,
    zone: HitZone,
    direct_kia: bool,
) -> Option<KillReason> {
    if hp_after <= OVERKILL_HP {
        return Some(KillReason::Overkill);
    }
    if direct_kia {
        return Some(KillReason::Forced);
    }
    match kind {
        DamageKind::Explosion => Some(KillReason::Explosion),
        DamageKind::Vehicle => Some(KillReason::Vehicle),
        DamageKind::Bullet if zone == HitZone::Head => Some(KillReason::Headshot),
        _ => None,
    }
}
