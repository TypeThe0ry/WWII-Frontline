//! Static map restrictions shared by authoritative placement and preview.

use bevy_ecs::prelude::Resource;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoBuildZoneError {
    /// Zero-based index in the supplied map data.
    pub polygon: usize,
}

impl std::fmt::Display for NoBuildZoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid no-build polygon at index {}", self.polygon)
    }
}

impl std::error::Error for NoBuildZoneError {}

#[derive(Clone, Debug, Default, Resource)]
pub struct NoBuildZones(Vec<Vec<[f64; 2]>>);

impl NoBuildZones {
    /// Polygons are horizontal `[x, z]` rings, with optional repeated closing vertex.
    /// Rings must have at least three finite vertices, no zero-length edges and nonzero area.
    pub fn new(polygons: Vec<Vec<[f32; 2]>>) -> Result<Self, NoBuildZoneError> {
        let mut zones = Vec::with_capacity(polygons.len());
        for (polygon, mut ring) in polygons.into_iter().enumerate() {
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() < 3
                || !ring.iter().flatten().all(|v| v.is_finite())
                || edges(&ring).any(|(a, b)| a == b)
            {
                return Err(NoBuildZoneError { polygon });
            }
            let ring: Vec<_> = ring.into_iter().map(|p| p.map(f64::from)).collect();
            let area_twice: f64 = ring[1..]
                .windows(2)
                .map(|pair| side(ring[0], pair[0], pair[1]))
                .sum();
            if area_twice == 0.0 {
                return Err(NoBuildZoneError { polygon });
            }
            zones.push(ring);
        }
        Ok(Self(zones))
    }

    /// Zones are a union. Edges and vertices are prohibited, regardless of height.
    pub fn contains(&self, pos: Vec3) -> bool {
        let point = [f64::from(pos.x), f64::from(pos.z)];
        point.iter().all(|v| v.is_finite()) && self.0.iter().any(|ring| contains(ring, point))
    }
}

fn edges<T>(ring: &[T]) -> impl Iterator<Item = (&T, &T)> {
    ring.iter().zip(ring.iter().cycle().skip(1))
}

fn side([ax, az]: [f64; 2], [bx, bz]: [f64; 2], [x, z]: [f64; 2]) -> f64 {
    (bx - ax) * (z - az) - (bz - az) * (x - ax)
}

fn contains(ring: &[[f64; 2]], point: [f64; 2]) -> bool {
    let [x, z] = point;
    let mut inside = false;
    for (&a, &b) in edges(ring) {
        let [ax, az] = a;
        let [bx, bz] = b;
        let side = side(a, b, point);
        if side == 0.0 && x >= ax.min(bx) && x <= ax.max(bx) && z >= az.min(bz) && z <= az.max(bz) {
            return true;
        }
        // Use half-open Z intervals so a ray through a vertex counts it only once.
        if (az > z) != (bz > z) && (side > 0.0) == (bz > az) {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concave_zones_include_edges_and_vertices_in_either_winding() {
        let mut ring = vec![[0., 0.], [4., 0.], [4., 4.], [2., 2.], [0., 4.], [0., 0.]];
        for _ in 0..2 {
            let zones =
                NoBuildZones::new(vec![ring.clone(), vec![[-4., -4.], [-2., -4.], [-3., -2.]]])
                    .unwrap();
            for (x, z) in [(2., 1.), (0., 1.), (4., 4.), (3., 3.), (-3., -3.)] {
                assert!(zones.contains(Vec3::new(x, 300., z)), "{x}, {z}");
            }
            for (x, z) in [(2., 3.), (3., 3.001), (-1., 2.), (5., 0.)] {
                assert!(!zones.contains(Vec3::new(x, 0., z)), "{x}, {z}");
            }
            ring.reverse();
        }
    }

    #[test]
    fn invalid_polygons_report_their_index() {
        for ring in [
            vec![[0., 0.], [1., 1.]],
            vec![[0., 0.], [1., 1.], [2., 2.]],
            vec![[0., 0.], [1., 0.], [1., f32::NAN]],
            vec![[0., 0.], [1., 0.], [1., 0.], [1., 1.]],
        ] {
            let valid = vec![[0., 0.], [1., 0.], [1., 1.]];
            assert_eq!(
                NoBuildZones::new(vec![valid, ring]).unwrap_err(),
                NoBuildZoneError { polygon: 1 }
            );
        }
    }
}
