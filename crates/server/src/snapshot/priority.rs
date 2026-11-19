//! Priority accumulator and the candidate rules (ARCHITECTURE §6.5.4, R3).
//!
//! * `rate_hz(E, C) = base(dist) x vis x imp`, clamped to `[2, 20]` and further capped by the
//!   client's packet rate (so a 15 Hz client does not wind its accumulators up).
//! * Candidacy with hysteresis: enter at 400 m, stay up to 440 m. R3 (enemy information): an
//!   enemy is a candidate only when it is within 96 m, or within 400 m and inside the view cone
//!   widened by 30 degrees (half angle 70 + 30 = 100; it stays until 110). Friends need only the
//!   distance.
//! * Selection order is deterministic: entities whose accumulator reached 10 first (starvation
//!   protection), then by accumulator descending, ties by ascending id.

use frontline_sim::Vec3;

pub const ENTER_M: f32 = 400.0;
pub const LEAVE_M: f32 = 440.0;
/// Enemies inside this distance are always candidates (R3), with 10 % leave hysteresis.
pub const CLOSE_M: f32 = 96.0;
pub const CLOSE_LEAVE_M: f32 = 105.6;
pub const MIN_RATE_HZ: f32 = 2.0;
pub const MAX_RATE_HZ: f32 = 20.0;
pub const ACC_FORCE: f32 = 10.0;
pub const ACC_CAP: f32 = 20.0;
/// An `entity_remove` record is repeated in this many consecutive packets.
pub const REMOVE_REPEATS: u8 = 3;
/// Window (ticks) in which firing / being hit raises the importance of an entity.
pub const FIRING_TICKS: u32 = 6;
pub const HURT_TICKS: u32 = 30;
/// Eye height of the observer and aim point of a target, metres above the feet.
pub const EYE_H: f32 = 1.6;
pub const AIM_H: f32 = 1.0;

// cosines of the cone angles (horizontal angle between view direction and target bearing)
const COS_70: f32 = 0.342_020_15;
const COS_100: f32 = -0.173_648_18;
const COS_110: f32 = -0.342_020_15;
const COS_5: f32 = 0.996_194_7;

/// What the receiving client looks like to the AOI.
#[derive(Debug, Clone, Copy)]
pub struct Observer {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub team: u8,
    /// Orientation `turn_union_ms` ago (equal to the current one without history): the cone
    /// test uses the union of both so a fast turn does not make enemies pop in.
    pub prev_yaw: f32,
    pub prev_pitch: f32,
}

/// Tunables of the candidate rules (`data/rules/aoi.json`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AoiParams {
    pub enemy_max_m: f32,
    pub enemy_close_m: f32,
    pub friendly_max_m: f32,
    pub leave_max_m: f32,
    pub leave_close_m: f32,
    /// Cosine of the enter / leave half angle (`frustum_half + margin`, `+ leave_extra`).
    pub cos_enter: f32,
    pub cos_leave: f32,
    pub turn_union_ticks: u32,
    pub los_period_ticks: u32,
    pub los_min_dist_m: f32,
    pub vehicle_importance: f32,
}

impl Default for AoiParams {
    fn default() -> Self {
        AoiParams {
            enemy_max_m: ENTER_M,
            enemy_close_m: CLOSE_M,
            friendly_max_m: ENTER_M,
            leave_max_m: LEAVE_M,
            leave_close_m: CLOSE_LEAVE_M,
            cos_enter: COS_100,
            cos_leave: COS_110,
            turn_union_ticks: 3,
            los_period_ticks: 8,
            los_min_dist_m: 20.0,
            vehicle_importance: 1.5,
        }
    }
}

impl AoiParams {
    pub fn from_rules(r: &frontline_sim::data::defs::AoiRulesDef, tick_hz: f32) -> Self {
        let enter = (r.frustum_half_deg + r.frustum_margin_deg) as f32;
        let leave = enter + r.frustum_leave_extra_deg as f32;
        AoiParams {
            enemy_max_m: r.enemy_max_m as f32,
            enemy_close_m: r.enemy_close_m as f32,
            friendly_max_m: r.friendly_max_m as f32,
            leave_max_m: r.leave_max_m as f32,
            leave_close_m: r.leave_close_m as f32,
            cos_enter: enter.to_radians().cos(),
            cos_leave: leave.to_radians().cos(),
            turn_union_ticks: libm_round(r.turn_union_ms as f32 / 1000.0 * tick_hz),
            los_period_ticks: (r.los_check_s * f64::from(tick_hz)).ceil().max(1.0) as u32,
            los_min_dist_m: r.los_min_dist_m as f32,
            vehicle_importance: r.vehicle_importance_mult as f32,
        }
    }
}

fn libm_round(x: f32) -> u32 {
    x.round().max(0.0) as u32
}

impl Observer {
    fn fwd_h(&self) -> (f32, f32) {
        (self.yaw.sin(), self.yaw.cos())
    }
}

/// Geometry between one observer and one target.
#[derive(Debug, Clone, Copy)]
pub struct Relation {
    pub dist: f32,
    /// Cosine of the horizontal angle between the view direction and the target (1 = ahead).
    pub cos_h: f32,
    /// Cosine of the 3D angle between the view direction and the target's aim point, the
    /// larger of the current and the `turn_union` orientation (1 = dead ahead).
    pub cos3: f32,
    /// Target aim point lies within 5 degrees of the crosshair.
    pub crosshair: bool,
}

pub fn relation(o: &Observer, tpos: Vec3) -> Relation {
    let d = tpos - o.pos;
    let dist = d.length();
    let h = d.x.hypot(d.z);
    let (fx, fz) = o.fwd_h();
    let cos_h = if h < 0.05 {
        1.0
    } else {
        (d.x * fx + d.z * fz) / h
    };
    let eye = o.pos + Vec3::new(0.0, EYE_H, 0.0);
    let to = tpos + Vec3::new(0.0, AIM_H, 0.0) - eye;
    let l = to.length();
    let crosshair = if l < 0.05 {
        true
    } else {
        let cp = o.pitch.cos();
        let f = Vec3::new(o.yaw.sin() * cp, o.pitch.sin(), o.yaw.cos() * cp);
        f.dot(to) / l >= COS_5
    };
    let cos3_at = |yaw: f32, pitch: f32| {
        if l < 0.05 {
            1.0
        } else {
            let cp = pitch.cos();
            Vec3::new(yaw.sin() * cp, pitch.sin(), yaw.cos() * cp).dot(to) / l
        }
    };
    let cos3 = cos3_at(o.yaw, o.pitch).max(cos3_at(o.prev_yaw, o.prev_pitch));
    Relation {
        dist,
        cos_h,
        cos3,
        crosshair,
    }
}

/// Base rate by distance (Hz).
pub fn base_rate(d: f32) -> f32 {
    if d <= 32.0 {
        20.0
    } else if d <= 96.0 {
        10.0
    } else if d <= 192.0 {
        5.0
    } else {
        2.0
    }
}

/// Visibility factor: 1 inside the +-70 degree cone, 0.35 behind the observer beyond 96 m,
/// 0.5 elsewhere.
pub fn visibility(r: &Relation) -> f32 {
    if r.cos_h >= COS_70 {
        1.0
    } else if r.cos_h < COS_110 && r.dist > CLOSE_M {
        0.35
    } else {
        0.5
    }
}

/// Importance factor.
pub fn importance(r: &Relation, firing: bool, hurt: bool) -> f32 {
    let mut i = 1.0;
    if firing {
        i *= 2.0;
    }
    if hurt {
        i *= 2.0;
    }
    if r.crosshair {
        i *= 1.5;
    }
    i
}

/// `rate_hz`, before the per-client packet-rate cap.
pub fn rate_hz(r: &Relation, firing: bool, hurt: bool) -> f32 {
    (base_rate(r.dist) * visibility(r) * importance(r, firing, hurt))
        .clamp(MIN_RATE_HZ, MAX_RATE_HZ)
}

/// Rate of a vehicle: the player formula times the vehicle importance (1.5), still within
/// `[2, 20]` Hz.
pub fn vehicle_rate_hz(p: &AoiParams, r: &Relation, firing: bool, hurt: bool) -> f32 {
    (base_rate(r.dist) * visibility(r) * importance(r, firing, hurt) * p.vehicle_importance)
        .clamp(MIN_RATE_HZ, MAX_RATE_HZ)
}

/// Candidate test with hysteresis (`was`: the entity is currently in the client's view).
pub fn is_candidate(p: &AoiParams, same_team: bool, r: &Relation, was: bool) -> bool {
    let max_d = match (same_team, was) {
        (true, false) => p.friendly_max_m,
        (false, false) => p.enemy_max_m,
        (_, true) => p.leave_max_m,
    };
    if r.dist > max_d {
        return false;
    }
    if same_team {
        return true;
    }
    let close = if was {
        p.leave_close_m
    } else {
        p.enemy_close_m
    };
    let cone = if was { p.cos_leave } else { p.cos_enter };
    r.dist <= close || r.cos3 >= cone
}

/// Visibility for event routing (no hysteresis).
pub fn event_visible(p: &AoiParams, same_team: bool, r: &Relation) -> bool {
    is_candidate(p, same_team, r, false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotState {
    #[default]
    Absent,
    InRange,
    /// Left the view; `remove_left` more packets carry an `entity_remove`.
    Leaving,
}

/// Per (client, entity) bookkeeping.
#[derive(Debug, Clone, Copy, Default)]
pub struct Slot {
    pub state: SlotState,
    pub acc: f32,
    pub remove_left: u8,
    /// Evaluation counter of the last pass that saw the entity as a candidate.
    pub touched: u32,
    pub(super) los: super::occlusion::LosSample,
}

/// Accumulators of one client, indexed by wire entity id (player id, 0..=127).
#[derive(Debug, Clone)]
pub struct PriorityState {
    pub slots: Vec<Slot>,
    pub last_eval_tick: Option<u32>,
    pub eval_counter: u32,
}

impl Default for PriorityState {
    fn default() -> Self {
        PriorityState {
            slots: vec![Slot::default(); 256],
            last_eval_tick: None,
            eval_counter: 0,
        }
    }
}

/// One selectable entity.
#[derive(Debug, Clone, Copy)]
pub struct Cand {
    pub id: u8,
    pub acc: f32,
}

/// Keeps entities with `acc >= 1` and sorts them: forced (`acc >= 10`) first, then by `acc`
/// descending, then by id.
pub fn order(c: &mut Vec<Cand>) {
    c.retain(|x| x.acc >= 1.0);
    c.sort_by(|a, b| {
        let fa = a.acc >= ACC_FORCE;
        let fb = b.acc >= ACC_FORCE;
        fb.cmp(&fa)
            .then(b.acc.total_cmp(&a.acc))
            .then(a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: AoiParams = AoiParams {
        enemy_max_m: ENTER_M,
        enemy_close_m: CLOSE_M,
        friendly_max_m: ENTER_M,
        leave_max_m: LEAVE_M,
        leave_close_m: CLOSE_LEAVE_M,
        cos_enter: COS_100,
        cos_leave: COS_110,
        turn_union_ticks: 3,
        los_period_ticks: 8,
        los_min_dist_m: 20.0,
        vehicle_importance: 1.5,
    };

    fn obs() -> Observer {
        Observer {
            pos: Vec3::new(100.0, 0.0, 100.0),
            yaw: 0.0, // facing +z
            pitch: 0.0,
            team: 0,
            prev_yaw: 0.0,
            prev_pitch: 0.0,
        }
    }

    fn at(dx: f32, dz: f32) -> Relation {
        relation(&obs(), obs().pos + Vec3::new(dx, 0.0, dz))
    }

    #[test]
    fn base_rate_bands() {
        assert_eq!(base_rate(10.0), 20.0);
        assert_eq!(base_rate(32.0), 20.0);
        assert_eq!(base_rate(33.0), 10.0);
        assert_eq!(base_rate(96.0), 10.0);
        assert_eq!(base_rate(150.0), 5.0);
        assert_eq!(base_rate(192.0), 5.0);
        assert_eq!(base_rate(300.0), 2.0);
        assert_eq!(base_rate(400.0), 2.0);
    }

    #[test]
    fn visibility_cone_and_behind() {
        assert_eq!(visibility(&at(0.0, 50.0)), 1.0); // ahead
        assert_eq!(visibility(&at(50.0, 0.0)), 0.5); // 90 deg to the side
        assert_eq!(visibility(&at(0.0, -150.0)), 0.35); // behind, > 96 m
        assert_eq!(visibility(&at(0.0, -50.0)), 0.5); // behind but close
    }

    #[test]
    fn rate_is_clamped_to_two_and_twenty() {
        // far, behind: 2 x 0.35 -> floor 2
        assert_eq!(rate_hz(&at(0.0, -300.0), false, false), 2.0);
        // close, firing and hurt and crosshair: 20 x 2 x 2 x 1.5 -> cap 20
        assert_eq!(rate_hz(&at(0.0, 10.0), true, true), 20.0);
        // 150 m ahead: 5 Hz, firing doubles it
        let r = at(0.0, 150.0);
        assert_eq!(rate_hz(&r, false, false), 5.0 * 1.5); // crosshair (straight ahead)
        assert_eq!(rate_hz(&at(30.0, 150.0), true, false), 10.0);
    }

    #[test]
    fn r3_enemy_needs_cone_or_closeness() {
        // ahead at 300 m: yes
        assert!(is_candidate(&P, false, &at(0.0, 300.0), false));
        // behind at 300 m: no
        assert!(!is_candidate(&P, false, &at(0.0, -300.0), false));
        // behind at 80 m: yes (<= 96 m)
        assert!(is_candidate(&P, false, &at(0.0, -80.0), false));
        // behind at 120 m: no
        assert!(!is_candidate(&P, false, &at(0.0, -120.0), false));
        // 95 deg to the side at 300 m: inside the +30 degree margin
        let a = 95f32.to_radians();
        assert!(is_candidate(
            &P,
            false,
            &at(300.0 * a.sin(), 300.0 * a.cos()),
            false
        ));
        // 105 deg: outside for a new entity, still inside while already visible (hysteresis)
        let a = 105f32.to_radians();
        let r = at(300.0 * a.sin(), 300.0 * a.cos());
        assert!(!is_candidate(&P, false, &r, false));
        assert!(is_candidate(&P, false, &r, true));
        // friends only need distance
        assert!(is_candidate(&P, true, &at(0.0, -399.0), false));
        assert!(!is_candidate(&P, true, &at(0.0, -401.0), false));
    }

    #[test]
    fn distance_hysteresis_400_to_440() {
        let r = at(0.0, 420.0);
        assert!(!is_candidate(&P, true, &r, false));
        assert!(is_candidate(&P, true, &r, true));
        let r = at(0.0, 445.0);
        assert!(!is_candidate(&P, true, &r, true));
    }

    #[test]
    fn order_forces_starved_first_and_breaks_ties_by_id() {
        let mut v = vec![
            Cand { id: 5, acc: 1.2 },
            Cand { id: 2, acc: 1.2 },
            Cand { id: 9, acc: 0.9 },
            Cand { id: 7, acc: 10.5 },
            Cand { id: 8, acc: 3.0 },
        ];
        order(&mut v);
        assert_eq!(v.iter().map(|c| c.id).collect::<Vec<_>>(), vec![7, 8, 2, 5]);
    }

    #[test]
    fn cone_boundary_is_100_degrees_in_3d() {
        let at_deg = |deg: f32, pitch_deg: f32| {
            let a = deg.to_radians();
            let p = pitch_deg.to_radians();
            // target at 300 m on a ray `deg` away from facing (+z) in the horizontal plane,
            // lifted by `pitch_deg`
            let o = Observer { pitch: p, ..obs() };
            let d = Vec3::new(a.sin(), 0.0, a.cos()) * 300.0;
            relation(&o, o.pos + d - Vec3::new(0.0, EYE_H - AIM_H, 0.0))
        };
        assert!(is_candidate(&P, false, &at_deg(99.9, 0.0), false));
        assert!(!is_candidate(&P, false, &at_deg(100.1, 0.0), false));
        // Looking 60 degrees up the cone is a true 3D cone: a horizon target 105 deg off in
        // yaw is only 97 deg away (in), 120 deg off is 104.5 deg away (out).
        assert!(is_candidate(&P, false, &at_deg(105.0, 60.0), false));
        assert!(!is_candidate(&P, false, &at_deg(120.0, 60.0), false));
    }

    #[test]
    fn close_boundary_96m() {
        let r = |d: f32| at(0.0, -d);
        assert!(is_candidate(&P, false, &r(95.9), false));
        assert!(!is_candidate(&P, false, &r(96.1), false));
        // already visible: stays until 105.6 m
        assert!(is_candidate(&P, false, &r(100.0), true));
        assert!(!is_candidate(&P, false, &r(106.0), true));
    }

    #[test]
    fn turn_union_keeps_what_was_in_view_100ms_ago() {
        // Now facing away (yaw 180), 100 ms ago facing +z: an enemy at +z 300 m stays a
        // candidate through the union.
        let o = Observer {
            yaw: core::f32::consts::PI,
            prev_yaw: 0.0,
            ..obs()
        };
        let r = relation(&o, o.pos + Vec3::new(0.0, 0.0, 300.0));
        assert!(is_candidate(&P, false, &r, false));
        let o2 = Observer {
            prev_yaw: core::f32::consts::PI,
            ..o
        };
        let r2 = relation(&o2, o2.pos + Vec3::new(0.0, 0.0, 300.0));
        assert!(!is_candidate(&P, false, &r2, false));
    }

    #[test]
    fn vehicles_update_at_least_1_4x_as_often_at_100m() {
        let r = at(40.0, 92.0); // ~100 m, inside the cone, off the crosshair
        let player = rate_hz(&r, false, false);
        let vehicle = vehicle_rate_hz(&P, &r, false, false);
        assert!(vehicle >= 1.4 * player, "{vehicle} vs {player}");
    }

    #[test]
    fn oscillating_at_400m_flips_at_most_twice_in_a_minute() {
        let mut was = false;
        let mut flips = 0;
        for i in 0..(60 * 20) {
            let d = if i % 2 == 0 { 395.0 } else { 405.0 };
            let now = is_candidate(&P, true, &at(0.0, d), was);
            if now != was {
                flips += 1;
            }
            was = now;
        }
        assert!(flips <= 2, "{flips} flips");
        // leaving needs > 440 m
        assert!(!is_candidate(&P, true, &at(0.0, 441.0), true));
        assert!(is_candidate(&P, true, &at(0.0, 439.0), true));
    }
}
