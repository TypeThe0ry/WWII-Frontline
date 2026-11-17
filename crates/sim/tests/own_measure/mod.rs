//! Own-game re-measurement of the M0 batch (REFERENCE_MEASUREMENTS 7.4, M0-032).
//!
//! Every quantity is measured the way the recorded analysis defines it (event-to-event times,
//! steady-state speeds, shots needed to kill, ...), by driving the real simulation functions
//! (`step_player`, `step_weapon`, the projectile integrator, `projectile::damage`) from the input
//! side, **not** by reading the data fields back. `data_value` then carries the number that `/data`
//! currently adopts so that reviewers see measured-vs-configured side by side.
//!
//! Read-only use of `crates/sim`; shared with `own_remeasure.rs` (export) and `ref_regression.rs`.
#![allow(dead_code)]

use std::collections::BTreeMap;

use frontline_protocol::msg::input::buttons;
use frontline_sim::movement::AUX_WALK;
use frontline_sim::projectile::damage;
use frontline_sim::weapon::ballistics::{substep, BallisticsParams, SUBSTEPS};
use frontline_sim::weapon::spread::view_dir;
use frontline_sim::weapon::{Fsm, TICK_UNITS};
use frontline_sim::{step_player, GameData, HitZone, Stance, Vec3};
use serde::Serialize;

use crate::common::*;

pub const HZ: f64 = 30.0;
pub const RIFLES: [&str; 2] = ["us_m1_garand", "ger_kar98k"];

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub subject_id: String,
    pub item: String,
    pub unit: String,
    /// `None` = not measurable in the M0 simulation (feature absent); `notes` says why.
    pub median: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub n: usize,
    pub p95: Option<f64>,
    pub method: String,
    pub data_field: String,
    pub data_value: Option<f64>,
    pub notes: String,
    pub series: Vec<f64>,
}

impl Entry {
    fn unavailable(
        id: &str,
        subject: &str,
        item: &str,
        unit: &str,
        field: &str,
        why: &str,
    ) -> Self {
        Entry {
            id: id.into(),
            subject_id: subject.into(),
            item: item.into(),
            unit: unit.into(),
            median: None,
            min: None,
            max: None,
            n: 0,
            p95: None,
            method: "none (feature absent in M0 sim)".into(),
            data_field: field.into(),
            data_value: None,
            notes: why.into(),
            series: vec![],
        }
    }
}

fn stats(v: &mut [f64]) -> (f64, f64, f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = if v.len() % 2 == 1 {
        v[v.len() / 2]
    } else {
        0.5 * (v[v.len() / 2 - 1] + v[v.len() / 2])
    };
    let p95 = v[((v.len() as f64 * 0.95) as usize).min(v.len() - 1)];
    (med, v[0], *v.last().unwrap(), p95)
}

#[allow(clippy::too_many_arguments)]
fn entry(
    id: &str,
    subject: &str,
    item: &str,
    unit: &str,
    mut v: Vec<f64>,
    method: &str,
    field: &str,
    data_value: Option<f64>,
    notes: String,
) -> Entry {
    let n = v.len();
    let (med, lo, hi, p95) = stats(&mut v);
    Entry {
        id: id.into(),
        subject_id: subject.into(),
        item: item.into(),
        unit: unit.into(),
        median: Some(med),
        min: Some(lo),
        max: Some(hi),
        n,
        p95: Some(p95),
        method: method.into(),
        data_field: field.into(),
        data_value,
        notes,
        series: vec![],
    }
}

fn angle_deg(a: Vec3, b: Vec3) -> f64 {
    f64::from(libm::acosf(a.dot(b).clamp(-1.0, 1.0)).to_degrees())
}

// ------------------------------------------------------------------------------------ weapons

fn gaps_s(shots: &[u32]) -> Vec<f64> {
    shots
        .windows(2)
        .map(|p| f64::from(p[1] - p[0]) / HZ)
        .collect()
}

/// Ticks from pressing reload to the weapon being usable again (`reload_finished`).
fn reload_s(weapon: &str, shots_before: u32) -> f64 {
    let mut r = Rig::new(weapon, 6);
    for _ in 0..shots_before {
        r.step(BTN_FIRE);
        r.idle(50);
    }
    assert!(r.step(BTN_RELOAD).reload_started.is_some());
    let mut ticks = 1.0; // the press tick itself counts, as the key goes down at its start
    loop {
        if r.step(0).reload_finished {
            break;
        }
        ticks += 1.0;
        assert!(ticks < 1000.0);
    }
    ticks / HZ
}

fn deviations(weapon: &str, setup: &dyn Fn(&mut Rig), aim_first: bool, n: u32) -> Vec<f64> {
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        let mut r = Rig::new(weapon, 6);
        r.tick = i;
        setup(&mut r);
        if aim_first {
            for _ in 0..12 {
                r.step(BTN_AIM);
            }
        }
        let out = r.step(if aim_first {
            BTN_AIM | BTN_FIRE
        } else {
            BTN_FIRE
        });
        let s = out.shot.expect("shot");
        v.push(angle_deg(s.dir, view_dir(0.0, 0.0)));
    }
    v
}

fn weapon_entries(id: &str) -> Vec<Entry> {
    let data = GameData::embedded();
    let def = data.weapon(id).unwrap();
    let g = &def.gameplay;
    let mut out = Vec::new();
    let bolt = matches!(g.fire.mode, frontline_sim::data::defs::FireMode::Bolt);
    let key = |base: &str| format!("{base}.{id}");

    // W-01 / W-02 / W-13: longest sustained rate of the fastest clicker, 20 s.
    let mut r = Rig::new(id, 6);
    let shots = r.max_rate(600);
    let gaps = gaps_s(&shots);
    let rpm =
        60.0 * (shots.len() as f64 - 1.0) / (f64::from(shots[shots.len() - 1] - shots[0]) / HZ);
    out.push(entry(
        &key("W-01"),
        id,
        "实际射速（长连射平均）",
        "发/分",
        vec![rpm],
        "tick log of max-rate clicker, 20 s, N-1 intervals over the span",
        "weapons/{w}.json:fire.rpm",
        Some(g.fire.cap_rpm.or(g.fire.bolt_cycle_ms.map(|m| 60_000.0 / m)).unwrap()),
        format!(
            "{} shots in 20 s; semi/bolt weapons are limited by the pacing interval, so this is the cap, not a cyclic rate",
            shots.len()
        ),
    ));
    if !bolt {
        out.push(entry(
            &key("W-02"),
            id,
            "半自动最快点射间隔",
            "秒",
            gaps.clone(),
            "tick log, intervals between consecutive shots of the fastest clicker",
            "weapons/{w}.json:fire.semi_min_interval_s",
            g.fire.cap_rpm.map(|rpm| 60.0 / rpm),
            "30 Hz quantisation: 15 ticks".into(),
        ));
    } else {
        out.push(entry(
            &key("W-13"),
            id,
            "栓动单次循环时间",
            "秒",
            gaps.clone(),
            "tick log, fire -> next fire at the earliest",
            "weapons/{w}.json:bolt.cycle_s",
            g.fire.bolt_cycle_ms.map(|m| m / 1000.0),
            "45 ticks @30 Hz".into(),
        ));
    }
    // W-03 / W-04 reload (press -> can fire again).
    let cap = Rig::new(id, 6).wp.mag_capacity;
    out.push(entry(
        &key("W-03"),
        id,
        "空仓换弹时间",
        "秒",
        vec![reload_s(id, u32::from(cap))],
        "ticks from reload press to reload_finished (empty magazine)",
        "weapons/{w}.json:reload.empty_s",
        Some(g.reload.empty_ms / 1000.0),
        "whole-clip reload in M0".into(),
    ));
    out.push(entry(
        &key("W-04"),
        id,
        "战术换弹时间",
        "秒",
        vec![reload_s(id, 1)],
        "ticks from reload press to reload_finished (1 round fired)",
        "weapons/{w}.json:reload.tactical_s",
        Some(g.reload.tactical_ms / 1000.0),
        String::new(),
    ));
    out.push(Entry::unavailable(
        &key("W-05"),
        id,
        "逐发装填每发时间与装满总时间",
        "秒",
        "weapons/{w}.json:reload.per_round_s",
        "Per-round loading is M1-013; M0 reloads whole clips (data per_round_ms is not used yet)",
    ));
    out.push(Entry::unavailable(
        &key("W-06"),
        id,
        "打断换弹的最早可开火时刻",
        "秒",
        "weapons/{w}.json:reload.interruptible_after_s",
        "Reload cannot be interrupted in M0 (weapon_fsm: reload_cannot_be_restarted_or_swapped_midway)",
    ));
    // W-07 / W-08 ADS.
    let mut r = Rig::new(id, 6);
    let total = r.wp.ads_units;
    let mut ticks = 0.0;
    loop {
        r.step(BTN_AIM);
        ticks += 1.0;
        if r.w.ads_progress(total) >= 1.0 {
            break;
        }
    }
    out.push(entry(
        &key("W-07"),
        id,
        "开镜进入时间",
        "秒",
        vec![ticks / HZ],
        "ticks from AIM press to ads_progress == 1",
        "weapons/{w}.json:ads.in_s",
        Some(g.ads_ms / 1000.0),
        "sim state; the client camera/FOV blend is a presentation timing to record separately"
            .into(),
    ));
    r.step(0);
    out.push(entry(
        &key("W-08"),
        id,
        "开镜退出时间",
        "秒",
        vec![f64::from(r.w.ads_progress(total)) * 0.0],
        "AIM release resets ads progress",
        "weapons/{w}.json:ads.out_s",
        None,
        "M0 sim releases ADS instantly (0 s); client camera blend is presentation".into(),
    ));
    out.push(Entry::unavailable(
        &key("W-09"),
        id,
        "ADS 放大倍率/FOV",
        "度",
        "weapons/{w}.json:ads.fov_deg",
        "FOV is a client rendering value; the sim has none (measure with the client at M0-025)",
    ));
    for (wid, name, field) in [
        ("W-11", "切枪时间：拔出", "weapons/{w}.json:equip.draw_s"),
        ("W-12", "切枪时间：收起", "weapons/{w}.json:equip.holster_s"),
    ] {
        out.push(Entry::unavailable(
            &key(wid),
            id,
            name,
            "秒",
            field,
            "No weapon switching in M0 (single weapon, weapon_slot ignored)",
        ));
    }

    // W-15 / W-16 / W-18: recoil per shot over 200 runs x 10 shots.
    let kicks = |aim: bool| -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let mut by_idx_p = vec![Vec::new(); 10];
        let mut by_idx_y = vec![Vec::new(); 10];
        for run in 0..200u32 {
            let mut r = Rig::new(id, 6);
            r.tick = run * 1000;
            if aim {
                for _ in 0..12 {
                    r.step(BTN_AIM);
                }
            }
            let mut fired = 0;
            let mut i = 0u32;
            while fired < 10 && i < 2000 {
                let (p0, y0) = (r.w.recoil_pitch_deg, r.w.recoil_yaw_deg);
                let b = r.max_rate_input() | if aim { BTN_AIM } else { 0 };
                if r.step(b).shot.is_some() {
                    by_idx_p[fired].push(f64::from(r.w.recoil_pitch_deg - p0));
                    by_idx_y[fired].push(f64::from(r.w.recoil_yaw_deg - y0));
                    fired += 1;
                }
                if r.w.mag == 0 {
                    r.w.mag = r.wp.mag_capacity;
                    r.w.fsm = Fsm::Ready;
                }
                i += 1;
            }
        }
        (by_idx_p, by_idx_y)
    };
    let (hp, hy) = kicks(false);
    let (ap, _) = kicks(true);
    let mean = |v: &Vec<f64>| v.iter().sum::<f64>() / v.len() as f64;
    let all_p: Vec<f64> = hp.iter().flatten().copied().collect();
    let all_y: Vec<f64> = hy.iter().flatten().map(|x| x.abs()).collect();
    let all_ap: Vec<f64> = ap.iter().flatten().copied().collect();
    let mut e = entry(
        &key("W-15"),
        id,
        "后坐模式：竖直（前10发）",
        "度/发",
        all_p.clone(),
        "view-pitch delta across each shot, 200 runs x 10 shots (hip, standing)",
        "weapons/{w}.json:recoil.pattern_pitch_deg[]",
        Some(g.recoil.vkick_deg),
        "kick = vkick x U(0.85, 1.15), identical for every shot index (no pattern in M0)".into(),
    );
    e.series = hp.iter().map(mean).collect();
    out.push(e);
    let mut e = entry(
        &key("W-16"),
        id,
        "后坐模式：水平（前10发）",
        "度/发",
        all_y,
        "|view-yaw delta| across each shot, 200 runs x 10 shots",
        "weapons/{w}.json:recoil.pattern_yaw_deg[]",
        Some(g.recoil.hsigma_deg),
        "median of |yaw kick|; yaw kick is N(0, hsigma); signed series = per-index mean".into(),
    );
    e.series = hy.iter().map(mean).collect();
    out.push(e);
    // W-17: time until recoil is back to zero after a single shot.
    let mut r = Rig::new(id, 6);
    r.step(BTN_FIRE);
    let mut t = 0.0;
    while r.w.recoil_pitch_deg > 0.0 {
        r.step(0);
        t += 1.0;
        assert!(t < 400.0);
    }
    out.push(entry(
        &key("W-17"),
        id,
        "后坐恢复：停火后回正时间与方式",
        "秒",
        vec![t / HZ],
        "ticks from the shot until the view recoil offset is 0",
        "weapons/{w}.json:recoil.recovery_s",
        Some(0.12 + g.recoil.vkick_deg / g.recoil.recover_deg_s),
        "automatic linear return after a 120 ms delay (approx., kick jitter +-15 %)".into(),
    ));
    out.push(entry(
        &key("W-18"),
        id,
        "ADS 与腰射后坐倍率",
        "比值",
        vec![mean(&all_ap) / mean(&all_p)],
        "mean ADS kick / mean hip kick, 2000 shots each",
        "weapons/{w}.json:recoil.ads_mult",
        None,
        "no ADS recoil factor in M0 (stance factor only)".into(),
    ));

    // W-19..W-21 spread (angular deviation of the shot from the aim line).
    let n = 10_000;
    let still = deviations(id, &|_| {}, false, n);
    let mut e = entry(
        &key("W-19"),
        id,
        "腰射扩散",
        "度",
        still.clone(),
        "angular deviation of 10 000 hip shots from the aim line (median; P95 and max in p95/max)",
        "weapons/{w}.json:spread.hip_deg",
        Some(g.spread.hip_deg),
        "uniform draw in a cone of half angle hip_deg; median ~ 0.71 x half angle".into(),
    );
    e.series = vec![];
    out.push(e);
    let ads_st = deviations(id, &|_| {}, true, n);
    let ads_cr = deviations(
        id,
        &|r| {
            r.auth.stance = Stance::Crouch;
            r.auth.stance_blend = 1.0;
        },
        true,
        n,
    );
    let med = |v: &Vec<f64>| {
        let mut c = v.clone();
        stats(&mut c).0
    };
    out.push(entry(
        &key("W-20"),
        id,
        "开镜扩散（站/蹲/趴）",
        "度",
        ads_st,
        "angular deviation of 10 000 ADS shots, standing",
        "weapons/{w}.json:spread.ads_deg",
        Some(g.spread.ads_deg),
        format!(
            "crouching ADS median {:.4} deg; prone is M1-011",
            med(&ads_cr)
        ),
    ));
    let run = deviations(id, &|r| r.auth.vel = Vec3::new(0.0, 0.0, 4.4), false, 4000);
    let walk = deviations(id, &|r| r.auth.vel = Vec3::new(0.0, 0.0, 2.0), false, 4000);
    let crouch = deviations(
        id,
        &|r| {
            r.auth.stance = Stance::Crouch;
            r.auth.stance_blend = 1.0;
        },
        false,
        4000,
    );
    let base = med(&still);
    out.push(entry(
        &key("W-21"),
        id,
        "扩散随移动/姿态/连射的增长与恢复",
        "比值",
        vec![med(&run) / base],
        "median hip deviation while running (4.4 m/s) / standing still",
        "weapons/{w}.json:spread.move_mult",
        None,
        format!(
            "walk (2 m/s) x{:.3}, crouch x{:.3}; bloom per shot {} deg decays at {} deg/s",
            med(&walk) / base,
            med(&crouch) / base,
            g.recoil.bloom_per_shot_deg,
            g.recoil.bloom_decay_deg_s
        ),
    ));

    // W-22..W-25 ballistics: horizontal bore, the sim integrator at 30 Hz x 4 sub-steps.
    let bp = BallisticsParams::from_def(def);
    let h = 1.0 / (HZ as f32 * SUBSTEPS as f32);
    let at = |range: f32| -> (f64, f64) {
        let (mut p, mut v) = (Vec3::ZERO, Vec3::new(bp.muzzle_velocity_mps, 0.0, 0.0));
        let mut t = 0.0f32;
        loop {
            let (p0, _) = (p, v);
            substep(&mut p, &mut v, bp.drag_k_per_m, bp.gravity_mps2, h);
            t += h;
            if p.x >= range {
                let f = (range - p0.x) / (p.x - p0.x);
                return (
                    f64::from(-(p0.y + f * (p.y - p0.y))),
                    f64::from(t - h * (1.0 - f)),
                );
            }
        }
    };
    let (d200, t200) = at(200.0);
    let (d300, t300) = at(300.0);
    out.push(entry(
        &key("W-22"),
        id,
        "弹头速度（曳光飞行时间法，>= 200 m）",
        "米/秒",
        vec![200.0 / t200, 300.0 / t300],
        "v = d / t_flight at 200 m and 300 m (average velocity over the flight, as read from a recording)",
        "weapons/{w}.json:ballistics.muzzle_velocity_mps",
        Some(g.ballistics.muzzle_velocity_mps),
        format!(
            "time of flight 200 m {t200:.4} s, 300 m {t300:.4} s; the average is below the muzzle velocity because of drag"
        ),
    ));
    let drops: BTreeMap<u32, f64> = [100u32, 200, 300, 400, 500]
        .iter()
        .map(|d| (*d, at(*d as f32).0))
        .collect();
    let extra = format!(
        "400 m {:.3} m, 500 m {:.3} m; relative to the bore line, level shot",
        drops[&400], drops[&500]
    );
    for (wid, d) in [("W-23", 100u32), ("W-24", 200), ("W-25", 300)] {
        out.push(entry(
            &key(wid),
            id,
            &format!("下坠：{d} m 落点偏移"),
            "米",
            vec![drops[&d]],
            "integrated projectile height below the bore line at that range",
            &format!("weapons/{{w}}.json:ballistics.drop_table_m[{d}]"),
            None,
            if d == 300 {
                extra.clone()
            } else {
                String::new()
            },
        ));
    }
    let _ = (d200, d300);
    let mut e = Entry::unavailable(
        &key("W-26"),
        id,
        "弹头飞行形式（hitscan/弹丸）",
        "枚举",
        "weapons/{w}.json:ballistics.model",
        "Enumeration, no number: simulated projectile (semi-implicit Euler, 4 sub-steps/tick, drag k), not hitscan",
    );
    e.method = "code inspection / projectile spawn".into();
    out.push(e);
    for (wid, name) in [
        ("W-27", "准星晃动幅度与频率（ADS 站立，无屏息）"),
        ("W-28", "晃动随姿态变化"),
        ("W-29", "屏息：抑制比、持续、恢复"),
    ] {
        out.push(Entry::unavailable(
            &key(wid),
            id,
            name,
            "度",
            "weapons/{w}.json:sway.*",
            "Sway and breath holding are not implemented in M0 (no sway state in WeaponState)",
        ));
    }
    out.push(entry(
        &key("W-30"),
        id,
        "弹匣容量与携弹量",
        "发",
        vec![f64::from(Rig::new(id, 0).w.mag)],
        "magazine rounds of a fresh weapon",
        "weapons/{w}.json:mag.capacity",
        Some(f64::from(Rig::new(id, 0).wp.mag_capacity)),
        format!(
            "spare rounds with 6 spare magazines: {}; data mag_reserve {}",
            Rig::new(id, 6).w.reserve,
            g.mag_reserve
        ),
    ));

    // W-37 / H-01..03: shots to kill a full-health rifleman.
    let hp = data.role("rifleman").unwrap().health_hp as f32;
    let stk = |zone: HitZone, d: f32| (hp / damage(def, zone, d) - 1e-4).ceil().max(1.0) as f64;
    out.push(entry(
        &key("W-37"),
        id,
        "伤害：各距离所需发数与部位倍率",
        "发",
        vec![stk(HitZone::Torso, 100.0)],
        "ceil(health / damage(zone, distance)) from projectile::damage; torso at 100 m",
        "weapons/{w}.json:damage.falloff[]",
        None,
        format!(
            "torso {:.2} HP at 100 m, {:.2} HP at 300 m; see H-01",
            damage(def, HitZone::Torso, 100.0),
            damage(def, HitZone::Torso, 300.0)
        ),
    ));
    for d in [50.0f32, 150.0, 300.0] {
        out.push(entry(
            &format!("H-01.{id}@{}m", d as u32),
            id,
            "致死发数（躯干）",
            "发",
            vec![stk(HitZone::Torso, d)],
            "ceil(100 HP / damage(torso, d))",
            "weapons/{w}.json:damage.shots_to_kill[dist]",
            None,
            format!("damage {:.2} HP", damage(def, HitZone::Torso, d)),
        ));
    }
    out.push(entry(
        &key("H-02"),
        id,
        "头部致死发数",
        "发",
        vec![stk(HitZone::Head, 50.0)],
        "ceil(100 HP / damage(head, 50 m))",
        "weapons/{w}.json:damage.headshot_stk",
        None,
        format!("head multiplier {}", g.part_mult.head),
    ));
    out.push(entry(
        &key("H-03"),
        id,
        "四肢致死发数/伤害倍率",
        "发",
        vec![stk(HitZone::Limb, 50.0)],
        "ceil(100 HP / damage(limb, 50 m))",
        "weapons/{w}.json:damage.limb_mult",
        None,
        format!("limb multiplier {}", g.part_mult.limb),
    ));
    out
}

// ------------------------------------------------------------------------------------ movement

fn steady_speed(btn: u16, aux: u8, mx: i8, mz: i8) -> f64 {
    let col = flat(4, 5.0);
    let p = move_params();
    let mut s = settle(&col, &p, 20.0, 20.0);
    let c = cmd(mx, mz, 0, btn, aux);
    for _ in 0..20 {
        s = step_player(&s, &c, &p, &col);
    }
    let start = s.pos;
    for _ in 0..90 {
        s = step_player(&s, &c, &p, &col);
        assert!(s.grounded && !s.blocked);
    }
    let d = s.pos - start;
    f64::from(libm::sqrtf(d.x * d.x + d.z * d.z)) / 3.0
}

fn tick_speed(a: Vec3, b: Vec3) -> f64 {
    let d = b - a;
    f64::from(libm::sqrtf(d.x * d.x + d.z * d.z)) * HZ
}

fn movement_entries() -> Vec<Entry> {
    let role = GameData::embedded().role("rifleman").unwrap();
    let m = &role.movement;
    let sub = "rifleman";
    let mut out = Vec::new();
    let one =
        |id: &str,
         item: &str,
         unit: &str,
         v: Vec<f64>,
         method: &str,
         field: &str,
         dv: Option<f64>,
         notes: String| { entry(id, sub, item, unit, v, method, field, dv, notes) };
    let walk = steady_speed(0, AUX_WALK, 0, 127);
    let run = steady_speed(0, 0, 0, 127);
    let sprint = steady_speed(buttons::SPRINT, 0, 0, 127);
    let crouch = steady_speed(buttons::CROUCH, 0, 0, 127);
    out.push(one(
        "M-01",
        "行走速度",
        "米/秒",
        vec![walk],
        "displacement over 3 s after 20 warm-up ticks (flat ground)",
        "movement.json:walk.speed_mps",
        Some(m.walk_mps),
        String::new(),
    ));
    out.push(one(
        "M-02",
        "奔跑速度",
        "米/秒",
        vec![run],
        "same",
        "movement.json:run.speed_mps",
        Some(m.run_mps),
        String::new(),
    ));
    out.push(one(
        "M-03",
        "冲刺速度",
        "米/秒",
        vec![sprint],
        "same",
        "movement.json:sprint.speed_mps",
        Some(m.sprint_mps),
        "M0 has unlimited stamina".into(),
    ));
    out.push(one(
        "M-04",
        "蹲行速度",
        "米/秒",
        vec![crouch],
        "same (crouch, not walk-modified)",
        "movement.json:crouch.speed_mps",
        Some(m.crouch_mps),
        format!(
            "crouch+walk {:.3} m/s",
            steady_speed(buttons::CROUCH, AUX_WALK, 0, 127)
        ),
    ));
    out.push(Entry::unavailable(
        "M-05",
        sub,
        "匍匐速度",
        "米/秒",
        "movement.json:prone.speed_mps",
        "Prone is M1-011; the sim ignores the PRONE button (data prone_mps unused)",
    ));
    let back = steady_speed(0, 0, 0, -127) / run;
    let strafe = steady_speed(0, 0, 127, 0) / run;
    out.push(one(
        "M-06",
        "后退/侧移速度倍率",
        "比值",
        vec![back, strafe],
        "steady speed backwards / sideways divided by forward run speed",
        "movement.json:backward_mult",
        None,
        format!("min/max = the two ratios (backward {back:.3}, strafe {strafe:.3})"),
    ));

    // Stance transitions: slope of the linear blend (time = 1 / slope), the continuous equivalent.
    let col = flat(2, 5.0);
    let p = move_params();
    let trans = |down: bool| -> f64 {
        let mut s = settle(&col, &p, 20.0, 20.0);
        if !down {
            for _ in 0..20 {
                s = step_player(&s, &cmd(0, 0, 0, buttons::CROUCH, 0), &p, &col);
            }
        }
        let btn = if down { buttons::CROUCH } else { 0 };
        let b0 = s.stance_blend;
        for _ in 0..3 {
            s = step_player(&s, &cmd(0, 0, 0, btn, 0), &p, &col);
        }
        3.0 / HZ / f64::from((s.stance_blend - b0).abs())
    };
    let st = f64::from(p.stance_ticks) / HZ;
    out.push(one("M-12", "姿态切换：站->蹲", "秒", vec![trans(true)], "blend slope over 3 ticks (full transition = 1 / slope)", "movement.json:stance.stand_to_crouch_s", m.stance_transition_ms.map(|v| v / 1000.0), format!("linear blend, {:.3} ticks = {st:.3} s; the transition completes on whole tick {} ({:.3} s)", p.stance_ticks, f64::from(p.stance_ticks).ceil(), f64::from(p.stance_ticks).ceil() / HZ)));
    out.push(one(
        "M-13",
        "姿态切换：蹲->站",
        "秒",
        vec![trans(false)],
        "same",
        "movement.json:stance.crouch_to_stand_s",
        m.stance_transition_ms.map(|v| v / 1000.0),
        String::new(),
    ));
    out.push(Entry::unavailable(
        "M-14",
        sub,
        "姿态切换：站/蹲->趴；趴->站/蹲",
        "秒",
        "movement.json:stance.to_prone_s",
        "Prone is M1-011",
    ));
    out.push(Entry::unavailable(
        "M-17",
        sub,
        "翻越矮墙时间与最高可翻越高度",
        "秒",
        "movement.json:vault.time_s",
        "No vault in M0 (autostep only: max step height in data autostep_max_m)",
    ));

    // M-19 jump (idle, flat ground).
    let mut s = settle(&col, &p, 20.0, 20.0);
    let y0 = s.pos.y;
    let mut peak = 0.0f32;
    s = step_player(&s, &cmd(0, 0, 0, buttons::JUMP, 0), &p, &col);
    let mut air = 1.0;
    while !s.grounded && air < 200.0 {
        peak = peak.max(s.pos.y - y0);
        s = step_player(&s, &cmd(0, 0, 0, 0, 0), &p, &col);
        air += 1.0;
    }
    out.push(one(
        "M-19",
        "跳跃高度与滞空时间",
        "米",
        vec![f64::from(peak)],
        "max foot height above the take-off point, sampled per tick",
        "movement.json:jump.height_m",
        m.jump_height_m,
        format!(
            "air time {:.3} s ({} ticks); peak sampled at 30 Hz (<= 1 cm low)",
            air / HZ,
            air
        ),
    ));

    // M-24..26 acceleration, stop, reverse (run speed, flat).
    let col = flat(4, 5.0);
    let mut s = settle(&col, &p, 30.0, 30.0);
    let run_cmd = cmd(0, 127, 0, 0, 0);
    let mut sp = vec![0.0f64];
    let mut prev = s.pos;
    for _ in 0..40 {
        s = step_player(&s, &run_cmd, &p, &col);
        sp.push(tick_speed(prev, s.pos));
        prev = s.pos;
    }
    let steady = *sp.last().unwrap();
    let t95 = interp_cross(&sp, 0.95 * steady, true) / HZ;
    out.push(one(
        "M-24",
        "加速时间（静止->95 % 稳定速度）",
        "秒",
        vec![t95],
        "per-tick speed, linear interpolation of the 95 % crossing",
        "movement.json:accel.to_run_s",
        Some(f64::from(p.accel_time_s)),
        format!("steady {steady:.3} m/s; data accel_time_s is the 0 -> 100 % time, 95 % = 0.95 x"),
    ));
    // stop
    let stop_cmd = cmd(0, 0, 0, 0, 0);
    let mut sp2 = vec![steady];
    let mut dist = 0.0f64;
    let mut prev = s.pos;
    for _ in 0..40 {
        s = step_player(&s, &stop_cmd, &p, &col);
        let v = tick_speed(prev, s.pos);
        dist += v / HZ;
        sp2.push(v);
        prev = s.pos;
    }
    let t_stop = interp_cross(&sp2, 0.05 * steady, false) / HZ;
    out.push(one("M-25", "减速时间与滑行距离", "秒", vec![t_stop], "per-tick speed after releasing the key, 5 % crossing (interpolated)", "movement.json:accel.stop_s", Some(f64::from(p.decel_time_s)), format!("stopping distance {dist:.3} m; the 5 % crossing of a linear ramp is 0.95 x decel_time_s")));
    // reverse
    let mut s = settle(&col, &p, 30.0, 30.0);
    for _ in 0..40 {
        s = step_player(&s, &run_cmd, &p, &col);
    }
    let back_cmd = cmd(0, -127, 0, 0, 0);
    let mut vz = vec![f64::from(s.vel.z)];
    for _ in 0..40 {
        s = step_player(&s, &back_cmd, &p, &col);
        vz.push(f64::from(s.vel.z));
    }
    let v0 = vz[0];
    let flip = interp_cross(&vz, 0.0, false) / HZ;
    let neg: Vec<f64> = vz.iter().map(|v| -v).collect();
    let full = interp_cross(&neg, 0.95 * v0, true) / HZ;
    out.push(one(
        "M-26",
        "转向惯性：180 度反向时间",
        "秒",
        vec![flip],
        "time until the velocity along the old heading reaches 0 (direction flips)",
        "movement.json:accel.reverse_s",
        None,
        format!("95 % of the speed reached in the new direction after {full:.3} s"),
    ));
    out
}

/// Fractional index where `v` first crosses `level` (rising when `up`, falling otherwise).
fn interp_cross(v: &[f64], level: f64, up: bool) -> f64 {
    for i in 1..v.len() {
        let hit = if up { v[i] >= level } else { v[i] <= level };
        if hit {
            let (a, b) = (v[i - 1], v[i]);
            return (i - 1) as f64
                + if (b - a).abs() < 1e-12 {
                    1.0
                } else {
                    (level - a) / (b - a)
                };
        }
    }
    f64::NAN
}

pub fn measure_all() -> Vec<Entry> {
    let mut v = Vec::new();
    for id in RIFLES {
        v.extend(weapon_entries(id));
    }
    v.extend(movement_entries());
    let _ = TICK_UNITS;
    v
}
