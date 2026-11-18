//! Client-side R3 audit (M1-068b / E1-4): an independent check of what the server *sent* this client. The server counts its own
//! leaks with the same predicate it used to select records (`aoi_enemy_leak_total`), which cannot catch a wrong predicate; this
//! re-states the rule from the specification (ARCHITECTURE 6.5, R3) using only what the client knows: its own predicted position,
//! the orientations it commanded, and the enemy records it received.
//!
//! Rule (leave limits, the hard bound): an enemy record is allowed when the enemy is within `CLOSE_LEAVE_M` (96 m x 1.1), or
//! within `FAR_LEAVE_M` (400 m x 1.1) and inside `CONE_DEG` of some orientation held during the last `HIST_TICKS` ticks.
//! The client's own pose differs from the server's by the prediction lead (RTT/2 plus the input delay), and the server's orientation
//! history is shorter than the client's commanded one, so the audit is deliberately *looser* than the server: `pos_tol_m` is added to
//! the distances, `cone_tol_deg` to the cone, and `HIST_TICKS` is 24 (0.8 s) against the server's 8. A reported leak is therefore a
//! real violation, while a leak that stays inside the tolerance is not detected here (the exact bound is `aoi_audit.rs`).

use std::collections::VecDeque;

use frontline_protocol::snapshot::decoder::EntitiesSoA;
use frontline_protocol::snapshot::EntityKind;
use frontline_sim::Vec3;

pub const CLOSE_LEAVE_M: f32 = 96.0 * 1.1;
pub const FAR_LEAVE_M: f32 = 400.0 * 1.1;
pub const CONE_DEG: f32 = 110.0;
pub const HIST_TICKS: usize = 24;
const EYE_M: f32 = 1.6;
const TARGET_CHEST_M: f32 = 1.0;

#[derive(Debug, Clone, Default)]
pub struct AoiAudit {
    pub own_faction: u8,
    pub pos_tol_m: f32,
    pub cone_tol_deg: f32,
    hist: VecDeque<(f32, f32)>,
    pub records_checked: u64,
    /// Enemy records farther than `CLOSE_LEAVE_M` (the ones that exercise the view-cone rule).
    pub beyond_close: u64,
    pub leaks: u64,
    /// Snapshots not audited (dead, or within 2 s of a death / respawn / position correction).
    pub skipped: u64,
    /// Largest distance (m) of a leaked record, 0 when none.
    pub worst_leak_m: f32,
    /// Print every leak to stderr (`FL_AOI_TRACE=1`).
    pub trace: bool,
    /// Tick of the snapshot being checked (for traces).
    pub tick: u32,
}

impl AoiAudit {
    pub fn new(own_faction: u8, pos_tol_m: f32, cone_tol_deg: f32) -> Self {
        Self {
            own_faction,
            pos_tol_m,
            cone_tol_deg,
            hist: VecDeque::with_capacity(HIST_TICKS + 1),
            trace: std::env::var_os("FL_AOI_TRACE").is_some(),
            ..Self::default()
        }
    }

    /// Records the orientation (yaw, pitch in radians; yaw 0 = +z) commanded for one tick.
    pub fn push_orientation(&mut self, yaw: f32, pitch: f32) {
        if self.hist.len() == HIST_TICKS {
            self.hist.pop_front();
        }
        self.hist.push_back((yaw, pitch));
    }

    /// Whether an enemy at `dist` metres whose direction from the eye is `to` (unit) is allowed under the (loosened) rule.
    pub fn allowed(&self, dist: f32, to: Vec3) -> bool {
        if dist <= CLOSE_LEAVE_M + self.pos_tol_m {
            return true;
        }
        if dist > FAR_LEAVE_M + self.pos_tol_m {
            return false;
        }
        let cos_min = (CONE_DEG + self.cone_tol_deg).to_radians().cos();
        self.hist.iter().any(|&(y, p)| {
            let cp = p.cos();
            Vec3::new(y.sin() * cp, p.sin(), y.cos() * cp).dot(to) >= cos_min
        })
    }

    /// Audits one decoded snapshot. Allied and non-player records are skipped (faction = `flags` bit 7).
    pub fn check(&mut self, own_pos: Vec3, e: &EntitiesSoA) {
        let eye = own_pos + Vec3::new(0.0, EYE_M, 0.0);
        for i in 0..e.len() {
            if e.kind[i] != EntityKind::Player || (e.flags[i] >> 7) == self.own_faction {
                continue;
            }
            let p = Vec3::new(e.pos[i * 3], e.pos[i * 3 + 1], e.pos[i * 3 + 2]);
            let to = p + Vec3::new(0.0, TARGET_CHEST_M, 0.0) - eye;
            let len = to.length().max(1e-4);
            let dist = (p - own_pos).length();
            self.records_checked += 1;
            if dist > CLOSE_LEAVE_M {
                self.beyond_close += 1;
            }
            if !self.allowed(dist, to / len) {
                if self.trace {
                    let best = self
                        .hist
                        .iter()
                        .map(|&(y, p)| {
                            let cp = p.cos();
                            Vec3::new(y.sin() * cp, p.sin(), y.cos() * cp)
                                .dot(to / len)
                                .clamp(-1.0, 1.0)
                                .acos()
                                .to_degrees()
                        })
                        .fold(f32::MAX, f32::min);
                    eprintln!("[aoi-audit] tick {} leak id {} dist {:.1} m best angle {:.1} deg own {:?} enemy {:?} flags {:#x} yaw_last {:?}", self.tick, e.ids[i], dist, best, own_pos, p, e.flags[i], self.hist.back());
                }
                self.leaks += 1;
                self.worst_leak_m = self.worst_leak_m.max(dist);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audit_facing_z() -> AoiAudit {
        let mut a = AoiAudit::new(0, 3.0, 5.0);
        for _ in 0..HIST_TICKS {
            a.push_orientation(0.0, 0.0);
        }
        a
    }

    #[test]
    fn close_enemies_are_always_allowed() {
        let a = audit_facing_z();
        assert!(a.allowed(10.0, Vec3::new(0.0, 0.0, -1.0))); // directly behind
        assert!(a.allowed(CLOSE_LEAVE_M, Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn far_enemies_need_the_view_cone() {
        let a = audit_facing_z();
        assert!(a.allowed(300.0, Vec3::new(0.0, 0.0, 1.0)));
        assert!(!a.allowed(300.0, Vec3::new(0.0, 0.0, -1.0))); // behind, 300 m: leak
        assert!(!a.allowed(500.0, Vec3::new(0.0, 0.0, 1.0))); // beyond the far limit even when looking at it
                                                              // 112 deg off axis is inside the 5 deg tolerance of the 110 deg cone, 120 deg is not
        let at = |deg: f32| Vec3::new(deg.to_radians().sin(), 0.0, deg.to_radians().cos());
        assert!(a.allowed(300.0, at(112.0)));
        assert!(!a.allowed(300.0, at(120.0)));
    }

    #[test]
    fn a_recent_orientation_keeps_an_enemy_allowed_until_it_ages_out() {
        let mut a = AoiAudit::new(0, 0.0, 0.0);
        a.push_orientation(core::f32::consts::PI, 0.0); // looked behind once
        for _ in 0..HIST_TICKS - 1 {
            a.push_orientation(0.0, 0.0);
        }
        let behind = Vec3::new(0.0, 0.0, -1.0);
        assert!(a.allowed(300.0, behind)); // still within the history
        a.push_orientation(0.0, 0.0);
        assert!(!a.allowed(300.0, behind)); // aged out
    }
}
