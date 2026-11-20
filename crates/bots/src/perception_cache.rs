//! Shared spatial shortlist for visual perception. Rebuild once per world tick.
//!
//! Neither a cached player row nor a candidate is a sighting. Callers must still check
//! line of sight and recognition before using an enemy's position for decisions.

use frontline_sim::view::PlayerView;
use frontline_sim::{PlayerId, Team, Vec3, WorldView};

use crate::difficulty::{Difficulty, Tuning};
use crate::perception::{in_view_cone, view_distance, Contact, TORSO_Y};

const CELL_M: f64 = 64.0;
const MAX_CANDIDATES: usize = 32;

fn cell(v: f64) -> i32 {
    libm::floor(v / CELL_M) as i32
}

struct Row {
    cell: [i32; 2],
    player: PlayerView,
}

/// A snapshot of the current tick's alive players, indexed by team/cell and by ID.
pub struct PerceptionCache {
    teams: [Vec<Row>; 2],
    by_id: [Option<PlayerView>; PlayerId::MAX as usize + 1],
}

impl Default for PerceptionCache {
    fn default() -> Self {
        Self {
            teams: Default::default(),
            by_id: [None; PlayerId::MAX as usize + 1],
        }
    }
}

impl PerceptionCache {
    /// Replace the snapshot, reusing the spatial index's allocation.
    pub fn rebuild(&mut self, view: &dyn WorldView) {
        self.by_id.fill(None);
        for rows in &mut self.teams {
            rows.clear();
        }
        for &player in view.players() {
            self.by_id[usize::from(player.player)] = Some(player);
            self.teams[player.team as usize].push(Row {
                cell: [cell(f64::from(player.pos.x)), cell(f64::from(player.pos.z))],
                player,
            });
        }
        for rows in &mut self.teams {
            rows.sort_unstable_by_key(|row| row.cell);
        }
    }

    /// O(1) lookup of a current-tick row. This is not visibility-filtered.
    pub fn player(&self, id: PlayerId) -> Option<&PlayerView> {
        self.by_id[usize::from(id)].as_ref()
    }

    /// Replace `out` with up to 32 enemies in range and in the view cone, sorted by
    /// distance, then player ID. The caller still owns ray tests and random rolls.
    pub fn candidates(
        &self,
        me: &PlayerView,
        eye: Vec3,
        tuning: &Tuning,
        diff: &Difficulty,
        out: &mut Vec<Contact>,
    ) {
        out.clear();
        let range = view_distance(tuning, diff);
        if !range.is_finite() || range < 0.0 || !eye.is_finite() {
            return;
        }
        let range2 = range * range;
        // Round the broad phase outwards to cover f32 subtraction and squaring in the
        // exact test (including a rounded hit just across a cell boundary).
        let extent = libm::sqrt(f64::from(range2.next_up())) * (1.0 + f64::from(f32::EPSILON));
        let (x, z) = (f64::from(eye.x), f64::from(eye.z));
        let (min_x, max_x) = (cell(x - extent), cell(x + extent));
        let (min_z, max_z) = (cell(z - extent), cell(z + extent));
        let enemy = match me.team {
            Team::Us => Team::Ger,
            Team::Ger => Team::Us,
        };

        let rows = &self.teams[enemy as usize];
        let mut rows = &rows[rows.partition_point(|r| r.cell[0] < min_x)..];
        // Jump between occupied X columns and directly to the Z interval within each.
        // The query never walks empty cells, even with a very large view distance.
        while let Some(x) = rows.first().map(|r| r.cell[0]).filter(|&x| x <= max_x) {
            let next = rows.partition_point(|r| r.cell[0] == x);
            let column = &rows[..next];
            let start = column.partition_point(|r| r.cell[1] < min_z);
            let end = column.partition_point(|r| r.cell[1] <= max_z);
            for row in &column[start..end] {
                let p = &row.player;
                let dist2 = (p.pos + Vec3::Y * TORSO_Y - eye).length_squared();
                if p.player == me.player || !dist2.is_finite() || dist2 > range2 {
                    continue;
                }
                let dist = libm::sqrtf(dist2);
                if dist > range || !in_view_cone(eye, me.yaw, p.pos, tuning.fov_deg) {
                    continue;
                }
                let i = out.partition_point(|c| {
                    c.dist
                        .total_cmp(&dist)
                        .then(c.player.cmp(&p.player))
                        .is_lt()
                });
                if i < MAX_CANDIDATES {
                    if out.len() == MAX_CANDIDATES {
                        out.pop();
                    }
                    out.insert(
                        i,
                        Contact {
                            player: p.player,
                            pos: p.pos,
                            dist,
                        },
                    );
                }
            }
            rows = &rows[next..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perception::can_see;
    use frontline_sim::ids::MAX_PLAYER_ID;
    use frontline_sim::rng::{unit_f32, Pcg64Mcg, RngCore};
    use frontline_sim::view::{EntityStateView, NavData, ObjectiveView, Resources, VehicleView};
    use frontline_sim::EntityId;

    struct TestView(Vec<PlayerView>);

    impl WorldView for TestView {
        fn players(&self) -> &[PlayerView] {
            &self.0
        }
        fn vehicles(&self) -> &[VehicleView] {
            &[]
        }
        fn entity_state(&self, _: EntityId) -> Option<EntityStateView> {
            panic!("the cache must only read the player snapshot")
        }
        fn los(&self, _: Vec3, _: Vec3) -> bool {
            true // The brute-force reference uses the original scan without occluders.
        }
        fn height_at(&self, _: f32, _: f32) -> Option<f32> {
            None
        }
        fn team_resources(&self, _: Team) -> Resources {
            Resources::default()
        }
        fn objectives(&self) -> &[ObjectiveView] {
            &[]
        }
        fn nav_hint(&self) -> &NavData {
            &NavData {}
        }
    }

    fn player(id: PlayerId, team: Team, pos: Vec3) -> PlayerView {
        PlayerView {
            player: id,
            entity: EntityId::from(id) + 1,
            team,
            pos,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    fn brute_force(
        view: &TestView,
        me: &PlayerView,
        eye: Vec3,
        tuning: &Tuning,
        diff: &Difficulty,
    ) -> Vec<Contact> {
        let range = diff.view_distance_m.min(tuning.view_distance_cap_m);
        let mut out: Vec<_> = view
            .players()
            .iter()
            .filter(|p| p.team != me.team && p.player != me.player)
            .filter_map(|p| {
                let dist2 = (p.pos + Vec3::Y * TORSO_Y - eye).length_squared();
                if dist2 > range * range || !can_see(view, eye, me.yaw, p.pos, tuning, diff) {
                    return None;
                }
                Some(Contact {
                    player: p.player,
                    pos: p.pos,
                    dist: libm::sqrtf(dist2),
                })
            })
            .collect();
        out.sort_unstable_by(|a, b| a.dist.total_cmp(&b.dist).then(a.player.cmp(&b.player)));
        out.truncate(32);
        out
    }

    #[test]
    fn matches_brute_force_on_1000_seeded_worlds() {
        let mut rng = Pcg64Mcg::new(0x8f83_21c5);
        let mut cache = PerceptionCache::default();
        let mut tuning = Tuning::embedded();
        let mut diff = Difficulty::embedded_default();
        let mut out = Vec::new();
        for case in 0..1000 {
            let dense = case % 4 == 0;
            let count = if dense { 100 } else { 1 + rng.next_u32() % 100 };
            let origin = Vec3::new(-192.0, 0.0, 64.0);
            let mut view = TestView(Vec::new());
            // Scatter player IDs independently of entity order, within the sim's slot range.
            let slots = u32::from(MAX_PLAYER_ID) + 1;
            let first_id = rng.next_u32() % slots;
            for i in 0..count {
                let mut coord = || match case % 4 {
                    0 => unit_f32(&mut rng) * 12.0 - 6.0,
                    1 => unit_f32(&mut rng) * 4096.0 - 2048.0,
                    2 => {
                        let edge = (rng.next_u32() % 11) as f32 * 64.0 - 320.0;
                        match rng.next_u32() % 3 {
                            0 => edge.next_down(),
                            1 => edge,
                            _ => edge.next_up(),
                        }
                    }
                    _ => unit_f32(&mut rng) * 512.0 - 256.0,
                };
                let x = coord();
                let z = coord();
                // Do not translate exact boundary cases: addition can erase a one-ULP offset.
                let mut pos = Vec3::new(x, unit_f32(&mut rng) * 16.0 - 8.0, z);
                if dense {
                    pos += origin;
                }
                let team = if rng.next_u32() & 1 == 0 {
                    Team::Us
                } else {
                    Team::Ger
                };
                let mut row = player(((first_id + i) % slots) as PlayerId, team, pos);
                row.entity = (i + 1) as EntityId;
                row.yaw = unit_f32(&mut rng) * std::f32::consts::TAU;
                view.0.push(row);
            }

            cache.rebuild(&view);
            for id in PlayerId::MIN..=PlayerId::MAX {
                assert_eq!(cache.player(id), view.0.iter().find(|p| p.player == id));
            }
            tuning.fov_deg = [60.0, 120.0, 180.0, 360.0][case / 4 % 4];
            tuning.view_distance_cap_m = [64.0, 150.0, 220.0, 300.0][case / 16 % 4];
            diff.view_distance_m = [0.0, 64.0, 150.0, 220.0, 300.0][case / 64 % 5];
            for me in view.0.iter().step_by(7) {
                let eye = me.pos + Vec3::new(0.0, 1.65, 0.0);
                cache.candidates(me, eye, &tuning, &diff, &mut out);
                assert_eq!(
                    out,
                    brute_force(&view, me, eye, &tuning, &diff),
                    "case {case}, player {}",
                    me.player
                );
            }
        }
    }

    #[test]
    fn keeps_nearest_32_and_breaks_distance_ties_by_id() {
        let me = player(0, Team::Us, Vec3::ZERO);
        let eye = Vec3::Y * TORSO_Y;
        let mut view = TestView(vec![me]);
        for id in 1..=50 {
            // Same distance, different cells, IDs interleaved between cells.
            let x = if id % 2 == 0 { -3.0 } else { 3.0 };
            view.0.push(player(id, Team::Ger, Vec3::new(x, 0.0, 4.0)));
        }
        view.0.push(player(51, Team::Ger, Vec3::Z));
        view.0.push(player(52, Team::Ger, -Vec3::Z * 2.0)); // outside FOV
        view.0.push(player(53, Team::Us, Vec3::Z)); // friendly
        let mut cache = PerceptionCache::default();
        cache.rebuild(&view);
        let mut out = Vec::with_capacity(MAX_CANDIDATES);
        cache.candidates(
            &me,
            eye,
            &Tuning::embedded(),
            &Difficulty::embedded_default(),
            &mut out,
        );
        assert!(out
            .iter()
            .map(|c| c.player)
            .eq(std::iter::once(51).chain(1..=31)));
        assert_eq!(out.capacity(), MAX_CANDIDATES);

        view.0.clear();
        cache.rebuild(&view);
        cache.candidates(
            &me,
            eye,
            &Tuning::embedded(),
            &Difficulty::embedded_default(),
            &mut out,
        );
        assert!(out.is_empty());
        assert!(cache.player(51).is_none());
    }

    #[test]
    fn includes_rounded_range_hit_across_cell_boundary() {
        let mut me = player(1, Team::Ger, Vec3::X * 64.0);
        me.yaw = -std::f32::consts::FRAC_PI_2;
        let eye = me.pos + Vec3::Y * TORSO_Y;
        let mut diff = Difficulty::embedded_default();
        diff.view_distance_m = 64.0;
        let view = TestView(vec![
            me,
            player(2, Team::Us, Vec3::X * -0.000_001),
            player(3, Team::Us, Vec3::X * -0.000_01),
            player(4, Team::Us, Vec3::Y), // torso is >64 m away, feet X/Z are not
        ]);
        let mut cache = PerceptionCache::default();
        cache.rebuild(&view);
        let mut out = Vec::new();
        cache.candidates(&me, eye, &Tuning::embedded(), &diff, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].player, 2);
        assert_eq!(out[0].dist, 64.0);
    }
}
