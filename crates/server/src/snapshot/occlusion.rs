//! Conservative AOI occlusion against the authoritative static heightfield.

use frontline_sim::{CollisionData, EntityId, Vec3};

// Probe above and to both sides of the target. Require positive terrain hits with cover
// behind them. A grazing ray, missing chunk or buried endpoint is not evidence of cover.
const COVER_DEPTH_M: f32 = 2.0;
const COVER_MARGIN_M: f32 = 0.25;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct LosSample {
    sample: Option<(EntityId, u32, bool)>,
}

impl LosSample {
    pub(super) fn blocked(
        &mut self,
        entity: EntityId,
        now: u32,
        period: u32,
        query: impl FnOnce() -> bool,
    ) -> bool {
        if let Some((id, tick, blocked)) = self.sample {
            if id == entity && now.wrapping_sub(tick) < period {
                return blocked;
            }
        }
        let blocked = query();
        self.sample = Some((entity, now, blocked));
        blocked
    }

    pub(super) fn clear(&mut self) {
        self.sample = None;
    }
}

pub(super) fn terrain_occludes(
    terrain: &CollisionData,
    eye: Vec3,
    target: Vec3,
    radius: f32,
    height: f32,
) -> bool {
    let along = target - eye;
    let side = Vec3::new(along.z, 0.0, -along.x).normalize_or_zero() * radius;
    let top = target + Vec3::Y * height;
    // Confirm cover even from slightly above the observer's eye and around the target.
    let eye = eye + Vec3::Y * COVER_MARGIN_M;
    [top, top + side, top - side].into_iter().all(|end| {
        let d = end - eye;
        let len = d.length();
        let Some(hit) = terrain.raycast(eye, d, len) else {
            return false;
        };
        if hit.t <= COVER_DEPTH_M || hit.t + 2.0 * COVER_DEPTH_M >= len {
            return false;
        }
        [COVER_DEPTH_M * 0.5, COVER_DEPTH_M]
            .into_iter()
            .all(|depth| {
                let p = eye + d * ((hit.t + depth) / len);
                terrain
                    .height_at(p.x, p.z)
                    .is_some_and(|y| y >= p.y + COVER_MARGIN_M)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_both_results_expires_and_does_not_follow_a_reused_slot() {
        let mut cache = LosSample::default();
        let t = u32::MAX - 2;
        assert!(cache.blocked(1, t, 8, || true));
        assert!(cache.blocked(1, t.wrapping_add(7), 8, || panic!("cached")));
        assert!(!cache.blocked(1, t.wrapping_add(8), 8, || false));
        assert!(!cache.blocked(1, 6, 8, || panic!("cached")));
        assert!(cache.blocked(2, 6, 8, || true));
        cache.clear();
        assert!(!cache.blocked(2, 6, 8, || false));
    }
}
