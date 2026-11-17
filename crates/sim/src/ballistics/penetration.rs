//! Material penetration (GAMEDESIGN 6.7, M1-012).
//!
//! `P = pen_power`. For each layer on the ray: `need = thickness_mm * cost_per_mm`; `P < need`
//! (or the layer is thicker than its `stop_mm`) ends the shot at the layer's entry; otherwise
//! `P -= need`, the damage multiplier is scaled by `max(0.25, P / pen_power)` and the direction
//! is deflected by up to `deflect_deg`. At most [`MAX_PENETRATIONS`] layers are penetrated, so a
//! trace has at most [`MAX_SEGMENTS`] segments; the third layer always stops it. A shallow hit on
//! a hard material ricochets instead (see [`super::ricochet`]).

use glam::Vec3;

use super::materials::{MaterialId, MaterialTable};
use super::ricochet;
use crate::rng::Pcg64Mcg;
use crate::weapon::spread::sample_cone;

/// Layers a shot may pass through.
pub const MAX_PENETRATIONS: usize = 2;
/// Segments of a trace (= penetrations + 1).
pub const MAX_SEGMENTS: usize = MAX_PENETRATIONS + 1;
/// Floor of the per-layer damage multiplier.
pub const MIN_LAYER_MULT: f32 = 0.25;

/// Ammunition facts the trace needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenAmmo {
    pub pen_power: f32,
}

/// A slab of material crossed by a ray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub material: MaterialId,
    pub entry: Vec3,
    pub exit: Vec3,
    /// Unit surface normal at the entry point (pointing out of the slab).
    pub normal: Vec3,
}

impl Layer {
    pub fn thickness_mm(&self) -> f32 {
        (self.exit - self.entry).length() * 1000.0
    }
}

/// Source of material layers (building walls, props, water; the terrain surface is not a layer).
pub trait LayerWorld {
    /// The first layer the ray `origin + t dir` (`dir` unit) enters within `max_t` metres, or `None`.
    fn first_layer(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<Layer>;
}

/// One straight piece of the shot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceSegment {
    pub from: Vec3,
    pub to: Vec3,
    pub dir: Vec3,
    /// The layer that ended this segment (entered at `to`), if any.
    pub material: Option<MaterialId>,
    /// Surface normal at `to` when a layer ended it.
    pub normal: Option<Vec3>,
    /// Penetration points left when the segment starts.
    pub pen_start: f32,
    /// Damage multiplier accumulated at the segment's start.
    pub damage_mult: f32,
}

/// Outcome of [`trace_with_penetration`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenResult {
    pub segments: [Option<TraceSegment>; MAX_SEGMENTS],
    /// Damage multiplier of whatever the shot hits after the last segment (1 when nothing was crossed).
    pub dealt_mult: f32,
    /// The shot ended inside / at a layer (nothing behind it is hit).
    pub stopped: bool,
    /// Ricochet: the continuing ray (position, unit direction, speed factor).
    pub ricochet: Option<(Vec3, Vec3, f32)>,
    /// Penetration points left at the end.
    pub pen_left: f32,
}

impl PenResult {
    pub fn segment_count(&self) -> usize {
        self.segments.iter().flatten().count()
    }
}

/// Traces a shot from `origin` along the unit `dir` for at most `max_t` metres.
pub fn trace_with_penetration(
    w: &dyn LayerWorld,
    table: &MaterialTable,
    origin: Vec3,
    dir: Vec3,
    max_t: f32,
    ammo: &PenAmmo,
    rng: &mut Pcg64Mcg,
) -> PenResult {
    let mut res = PenResult {
        segments: [None; MAX_SEGMENTS],
        dealt_mult: 1.0,
        stopped: false,
        ricochet: None,
        pen_left: ammo.pen_power,
    };
    let (mut pos, mut dir, mut left) = (origin, dir.normalize_or_zero(), max_t);
    let mut p = ammo.pen_power;
    for i in 0..MAX_SEGMENTS {
        let layer = w.first_layer(pos, dir, left).and_then(|l| {
            let m = table.get(l.material)?;
            // Layers thinner than `min_mm` do not count (shallow water).
            (l.thickness_mm() >= m.min_mm).then_some(l)
        });
        let Some(layer) = layer else {
            res.segments[i] = Some(TraceSegment {
                from: pos,
                to: pos + dir * left,
                dir,
                material: None,
                normal: None,
                pen_start: p,
                damage_mult: res.dealt_mult,
            });
            break;
        };
        let m = *table.get(layer.material).expect("checked above");
        res.segments[i] = Some(TraceSegment {
            from: pos,
            to: layer.entry,
            dir,
            material: Some(layer.material),
            normal: Some(layer.normal),
            pen_start: p,
            damage_mult: res.dealt_mult,
        });
        if m.ricochet && ricochet::bounces(dir, layer.normal) {
            res.ricochet = Some((
                layer.entry,
                ricochet::reflect(dir, layer.normal),
                ricochet::speed_factor(),
            ));
            res.stopped = true;
            break;
        }
        let thickness = layer.thickness_mm();
        let need = thickness * m.cost_per_mm;
        let capped = m.stop_mm > 0.0 && thickness > m.stop_mm;
        if i == MAX_PENETRATIONS || capped || p < need {
            res.stopped = true;
            break;
        }
        p -= need;
        res.dealt_mult *= (p / ammo.pen_power).max(MIN_LAYER_MULT);
        // Continue behind the layer, slightly deflected.
        left -= (layer.exit - pos).length();
        pos = layer.exit;
        dir = sample_cone(rng, dir, m.deflect_deg);
    }
    res.pen_left = p;
    res
}
