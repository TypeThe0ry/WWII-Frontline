//! M1-014: damage model (zones, falloff, helmet, overkill, friendly fire), recent-damage window,
//! hitbox history ring.
mod common;

use common::*;
use frontline_sim::combat::credit::RecentDamage;
use frontline_sim::combat::damage::{
    compute_damage, DamageKind, DamageOutcome, HitInfo, KillReason, TargetCtx,
};
use frontline_sim::combat::lagcomp_history::{LagCompHistory, PlayerHitboxes};
use frontline_sim::weapon::hitbox::volumes;
use frontline_sim::{HitZone, Tick, Vec3};

fn hit(zone: HitZone, d: f32) -> HitInfo {
    HitInfo {
        zone,
        travelled_m: d,
        pen_mult: 1.0,
        kind: DamageKind::Bullet,
    }
}

fn foe(hp: f32) -> TargetCtx {
    TargetCtx {
        hp,
        same_team: false,
        helmet: true,
        friendly_fire: true,
    }
}

/// Shots until the target is no longer on its feet.
fn shots_to_down(id: &str, zone: HitZone, d: f32) -> u32 {
    let def = data_with(|_| {}).weapon(id).unwrap();
    let mut hp = 100.0;
    for n in 1..100 {
        let r = compute_damage(def, &hit(zone, d), &foe(hp));
        if r.outcome != DamageOutcome::Hit {
            return n;
        }
        hp = r.hp_after;
    }
    panic!("never down");
}

const TTK: [(&str, u32); 7] = [
    ("us_m1_garand", 2),
    ("ger_kar98k", 1),
    ("us_m1_carbine", 3),
    ("us_thompson_m1a1", 3),
    ("ger_mp40", 3),
    ("us_m1911a1", 3),
    ("ger_p38", 3),
];

#[test]
fn ttk_table() {
    for (id, n) in TTK {
        assert_eq!(shots_to_down(id, HitZone::Torso, 5.0), n, "{id}");
    }
    // Snapshot of the whole table (near / mid / far) so data changes show up in review.
    let mut lines = Vec::new();
    for (id, _) in TTK {
        let def = data_with(|_| {}).weapon(id).unwrap();
        let f1 = def.gameplay.falloff.last().unwrap().dist_m as f32;
        lines.push(format!(
            "{id}: near {} mid {} far {}",
            shots_to_down(id, HitZone::Torso, 5.0),
            shots_to_down(id, HitZone::Torso, f1 * 0.5),
            shots_to_down(id, HitZone::Torso, f1),
        ));
    }
    let got = lines.join("\n") + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/ttk.txt");
    if std::env::var("UPDATE_SNAPSHOT").is_ok() {
        std::fs::write(&path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(&path).unwrap(), "ttk snapshot");
}

#[test]
fn falloff_midrange_and_far() {
    let d = data_with(|_| {});
    let g = d.weapon("us_m1_garand").unwrap();
    let r = compute_damage(g, &hit(HitZone::Torso, 600.0), &foe(100.0));
    assert!((r.damage - 50.4).abs() < 0.01);
    assert_eq!(shots_to_down("us_m1_garand", HitZone::Torso, 600.0), 2);
    let c = d.weapon("us_m1_carbine").unwrap();
    let r = compute_damage(c, &hit(HitZone::Torso, 250.0), &foe(100.0));
    assert!((r.damage - 26.4).abs() < 0.01);
    assert_eq!(shots_to_down("us_m1_carbine", HitZone::Torso, 250.0), 4);
    // Beyond F1 the minimum multiplier holds.
    let r = compute_damage(c, &hit(HitZone::Torso, 900.0), &foe(100.0));
    assert!((r.damage - 26.4).abs() < 0.01);
}

#[test]
fn head_rules() {
    let d = data_with(|_| {});
    let r = compute_damage(
        d.weapon("us_m1_garand").unwrap(),
        &hit(HitZone::Head, 5.0),
        &foe(100.0),
    );
    assert!((r.damage - 210.0).abs() < 0.01);
    assert_eq!(r.outcome, DamageOutcome::Killed(KillReason::Overkill));
    // MP40 against a helmet: 36 x 2.0 = 72, no one-shot kill; without a helmet 108.
    let mp = d.weapon("ger_mp40").unwrap();
    let r = compute_damage(mp, &hit(HitZone::Head, 5.0), &foe(100.0));
    assert!((r.damage - 72.0).abs() < 0.01);
    assert_eq!(r.outcome, DamageOutcome::Hit);
    let bare = TargetCtx {
        helmet: false,
        ..foe(100.0)
    };
    assert!((compute_damage(mp, &hit(HitZone::Head, 5.0), &bare).damage - 108.0).abs() < 0.01);
    // A rifle ignores the helmet.
    let g = d.weapon("us_m1_garand").unwrap();
    assert_eq!(
        compute_damage(g, &hit(HitZone::Head, 5.0), &bare).damage,
        compute_damage(g, &hit(HitZone::Head, 5.0), &foe(100.0)).damage
    );
    // A bullet to the head that only takes hp to 0..-60 kills; to the torso it downs.
    let k = d.weapon("ger_kar98k").unwrap();
    let r = compute_damage(k, &hit(HitZone::Head, 5.0), &foe(100.0));
    assert!(matches!(r.outcome, DamageOutcome::Killed(_)));
}

#[test]
fn overkill_and_downed() {
    let d = data_with(|_| {});
    let k = d.weapon("ger_kar98k").unwrap(); // 100 to the torso
    let r = compute_damage(k, &hit(HitZone::Torso, 5.0), &foe(30.0));
    assert_eq!((r.hp_after, r.overkill), (-70.0, 70.0));
    assert_eq!(r.outcome, DamageOutcome::Killed(KillReason::Overkill));
    let mut weak = k.clone();
    weak.gameplay.damage_torso_hp = 60.0;
    let r = compute_damage(&weak, &hit(HitZone::Torso, 5.0), &foe(30.0));
    assert_eq!(r.hp_after, -30.0);
    assert_eq!(r.outcome, DamageOutcome::Downed);
    // Explosions kill instead of downing.
    let mut e = hit(HitZone::Torso, 5.0);
    e.kind = DamageKind::Explosion;
    let r = compute_damage(&weak, &e, &foe(30.0));
    assert_eq!(r.outcome, DamageOutcome::Killed(KillReason::Explosion));
}

#[test]
fn friendly_fire_coefficients() {
    let g = data_with(|_| {}).weapon("us_m1_garand").unwrap();
    let ally = TargetCtx {
        same_team: true,
        ..foe(100.0)
    };
    assert!((compute_damage(g, &hit(HitZone::Torso, 5.0), &ally).damage - 35.0).abs() < 0.01);
    let mut m = hit(HitZone::Torso, 5.0);
    m.kind = DamageKind::Melee;
    assert_eq!(compute_damage(g, &m, &ally).damage, 0.0);
    let off = TargetCtx {
        friendly_fire: false,
        ..ally
    };
    assert_eq!(
        compute_damage(g, &hit(HitZone::Torso, 5.0), &off).damage,
        0.0
    );
    let mut pen = hit(HitZone::Torso, 5.0);
    pen.pen_mult = 0.5;
    assert!((compute_damage(g, &pen, &foe(100.0)).damage - 35.0).abs() < 0.01);
}

#[test]
fn stance_volumes_heights() {
    let top = |b: f32| {
        volumes(b)
            .iter()
            .map(|v| v.y1.max(v.y0) + v.r)
            .fold(0.0f32, f32::max)
    };
    // Head sphere top: the standing / crouched / prone body heights of GAMEDESIGN 9.5 within the
    // capsule radius (PI-053 records the exact volumes).
    assert!((top(0.0) - 1.79).abs() < 0.02, "{}", top(0.0));
    assert!((top(1.0) - 1.19).abs() < 0.02, "{}", top(1.0));
    assert!((top(2.0) - 0.51).abs() < 0.02, "{}", top(2.0));
}

#[test]
fn recent_damage_window() {
    let mut r = RecentDamage::default();
    let window = frontline_sim::score::ScoreRules::embedded().assist_s * 30;
    r.record(3, 40.0, Tick(0), window);
    r.record(4, 20.0, Tick(100), window);
    r.record(3, 10.0, Tick(200), window);
    assert_eq!(r.total_from(3), 50.0);
    assert_eq!(
        r.attackers_within(Tick(200), window),
        vec![(3, 50.0), (4, 20.0)]
    );
    assert_eq!(r.last_hit().unwrap().attacker, 3);
    r.record(5, 1.0, Tick(window + 1), window);
    assert_eq!(r.total_from(3), 10.0);
    r.clear();
    assert!(r.is_empty());
}

fn pose(id: u16, x: f32) -> PlayerHitboxes {
    PlayerHitboxes {
        id,
        foot: Vec3::new(x, 0.0, 0.0),
        blend: 0.0,
        head_dx: 0.0,
        head_dz: 0.0,
    }
}

#[test]
fn history_ring_wraps_and_interpolates() {
    let mut h = LagCompHistory::new();
    for t in 0..40u32 {
        h.record(Tick(t), &[pose(1, t as f32)]);
    }
    assert_eq!(h.len(), 16);
    assert!(h.frame(Tick(23)).is_none(), "overwritten");
    assert!(h.frame(Tick(24)).is_some());
    let mid = h.hitboxes_at(Tick(30), 0.5).unwrap();
    assert!((mid[0].foot.x - 30.5).abs() < 1e-4);
    assert!(h.hitboxes_at(Tick(5), 0.0).is_none());
    // The newest frame has no successor: frac is ignored.
    let last = h.hitboxes_at(Tick(39), 0.5).unwrap();
    assert!((last[0].foot.x - 39.0).abs() < 1e-4);
}
