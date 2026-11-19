//! Versioned canonical building geometry shared by authority and prediction.

use std::collections::BTreeMap;
use std::fmt;

use glam::{Vec2, Vec3};
use rapier3d::prelude::{Collider, ColliderBuilder, TriMeshFlags, Vector};
use serde::Deserialize;

use crate::hash::HashWriter;

/// Bounds parsing and allocations for browser-supplied data, including ignored fields.
pub const MAX_BUILDING_COLLISION_BYTES: usize = 4 * 1024 * 1024;
const MAX_BUILDINGS: usize = 512;
const MAX_COLLIDERS: usize = 4096;
const MAX_VERTICES: usize = 65_536;
const MAX_TRIANGLES: usize = 65_536;
const MAX_COORD_M: f32 = 16_384.0;

#[derive(Debug)]
pub struct BuildingCollisionError(String);

impl fmt::Display for BuildingCollisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BuildingCollisionError {}

/// Validated wall boxes and ready-to-insert colliders, in building ID and then document order.
#[derive(Clone, Default)]
pub struct BuildingCollision {
    walls: BTreeMap<String, Vec<BuildingWall>>,
    colliders: Vec<Collider>,
    digest: u64,
}

impl BuildingCollision {
    /// Parses and compiles the entire document before the caller installs any of it.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BuildingCollisionError> {
        if bytes.len() > MAX_BUILDING_COLLISION_BYTES {
            return Err(BuildingCollisionError(format!(
                "building collision exceeds {MAX_BUILDING_COLLISION_BYTES} bytes"
            )));
        }
        let mut raw: Document =
            serde_json::from_slice(bytes).map_err(|e| BuildingCollisionError(e.to_string()))?;
        if raw.version != 1 {
            return Err(BuildingCollisionError(format!(
                "unsupported building collision version {}",
                raw.version
            )));
        }
        if raw.buildings.len() > MAX_BUILDINGS {
            return Err(BuildingCollisionError("too many buildings".into()));
        }
        raw.buildings.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        let mut out = Self::default();
        let mut hash = HashWriter::new();
        hash.write_u32(raw.buildings.len() as u32);
        let (mut total_vertices, mut total_triangles) = (0, 0);
        for building in raw.buildings {
            let fail =
                |msg: &str| BuildingCollisionError(format!("building {:?}: {msg}", building.id));
            if building.id.is_empty() || building.id.len() > 128 {
                return Err(BuildingCollisionError(
                    "building ID must contain 1 to 128 bytes".into(),
                ));
            }
            if out.walls.contains_key(&building.id) {
                return Err(fail("duplicate ID"));
            }
            if building.walls.is_empty() || building.surfaces.is_empty() {
                return Err(fail("missing walls or surfaces"));
            }
            if out.colliders.len() + building.walls.len() + building.surfaces.len() > MAX_COLLIDERS
            {
                return Err(fail("too many building colliders"));
            }
            hash.write_u32(building.id.len() as u32);
            hash.write_bytes(building.id.as_bytes());
            hash.write_u32(building.walls.len() as u32);
            let mut walls = Vec::with_capacity(building.walls.len());
            for wall in building.walls {
                let values = [
                    wall.a[0],
                    wall.a[1],
                    wall.b[0],
                    wall.b[1],
                    wall.bottom,
                    wall.top,
                    wall.width,
                ];
                if !values.into_iter().all(bounded) {
                    return Err(fail("wall coordinates or width out of bounds"));
                }
                let wall = BuildingWall::new(wall.a, wall.b, wall.bottom, wall.top, wall.width)
                    .filter(|w| {
                        w.half.cmple(Vec3::new(256.0, 64.0, 4.0)).all()
                            && w.half.min_element() >= 0.0005
                    })
                    .ok_or_else(|| fail("degenerate wall"))?;
                for v in values {
                    hash.write_f32(v);
                }
                out.colliders.push(wall.collider());
                walls.push(wall);
            }
            hash.write_u32(building.surfaces.len() as u32);
            for surface in building.surfaces {
                total_vertices += surface.vertices.len();
                total_triangles += surface.indices.len() / 3;
                if total_vertices > MAX_VERTICES || total_triangles > MAX_TRIANGLES {
                    return Err(fail("too many surface vertices or triangles"));
                }
                if surface.vertices.len() < 3
                    || surface.indices.is_empty()
                    || !surface.indices.len().is_multiple_of(3)
                {
                    return Err(fail("invalid triangle buffers"));
                }
                hash.write_u8(surface.kind as u8);
                hash.write_u32(surface.vertices.len() as u32);
                let vertices: Vec<Vector> = surface
                    .vertices
                    .into_iter()
                    .map(|v| {
                        for x in v {
                            if !bounded(x) {
                                return Err(fail("surface vertex out of bounds"));
                            }
                            hash.write_f32(x);
                        }
                        Ok(Vector::from_array(v))
                    })
                    .collect::<Result<_, _>>()?;
                hash.write_u32(surface.indices.len() as u32);
                let triangles = surface
                    .indices
                    .chunks_exact(3)
                    .map(|ix| {
                        let tri = [ix[0], ix[1], ix[2]];
                        if tri.iter().any(|&i| i as usize >= vertices.len()) {
                            return Err(fail("triangle index out of bounds"));
                        }
                        let [a, b, c] = tri.map(|i| vertices[i as usize]);
                        if (b - a).cross(c - a).length_squared() <= 1e-12 {
                            return Err(fail("degenerate triangle"));
                        }
                        for i in tri {
                            hash.write_u32(i);
                        }
                        Ok(tri)
                    })
                    .collect::<Result<_, _>>()?;
                // Internal-edge normals avoid catching the KCC on coplanar triangulation seams.
                let mesh = ColliderBuilder::trimesh_with_flags(
                    vertices,
                    triangles,
                    TriMeshFlags::FIX_INTERNAL_EDGES_TWO_SIDED,
                )
                .map_err(|e| fail(&format!("invalid triangle mesh: {e}")))?;
                out.colliders
                    .push(mesh.user_data(BUILDING_COLLIDER).build());
            }
            out.walls.insert(building.id, walls);
        }
        out.digest = hash.finish();
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.walls.is_empty()
    }

    /// Wall boxes for an ID, suitable for the server's material-tagged cover queries.
    pub fn walls(&self, id: &str) -> Option<&[BuildingWall]> {
        self.walls.get(id).map(Vec::as_slice)
    }

    pub fn buildings(&self) -> impl Iterator<Item = (&str, &[BuildingWall])> {
        self.walls
            .iter()
            .map(|(id, walls)| (id.as_str(), walls.as_slice()))
    }

    pub(super) fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.colliders.iter().cloned()
    }

    pub(super) fn hash_into(&self, h: &mut HashWriter) {
        h.write_bytes(b"building-collision-v1");
        h.write_u64(self.digest);
    }
}

pub(super) const BUILDING_COLLIDER: u128 = 1;

/// An upright oriented box. Cover and Rapier use the same centre, axis and half-extents.
#[derive(Debug, Clone, Copy)]
pub struct BuildingWall {
    pub(super) center: Vec3,
    pub(super) axis: Vec2,
    pub(super) half: Vec3,
}

impl BuildingWall {
    pub(super) fn new(a: [f32; 2], b: [f32; 2], bottom: f32, top: f32, width: f32) -> Option<Self> {
        let [a, b] = [a, b].map(Vec2::from_array);
        let edge = b - a;
        let length = edge.length();
        let height = top - bottom;
        if !length.is_finite()
            || length < 0.001
            || !height.is_finite()
            || height <= 0.0
            || !width.is_finite()
            || width <= 0.0
        {
            return None;
        }
        let mid = a + edge * 0.5;
        Some(Self {
            center: Vec3::new(mid.x, bottom + height * 0.5, mid.y),
            axis: edge / length,
            half: Vec3::new(length * 0.5, height * 0.5, width * 0.5),
        })
    }

    fn collider(self) -> Collider {
        let yaw = libm::atan2f(-self.axis.y, self.axis.x);
        ColliderBuilder::cuboid(self.half.x, self.half.y, self.half.z)
            .translation(Vector::from_array(self.center.to_array()))
            .rotation(Vector::new(0.0, yaw, 0.0))
            .user_data(BUILDING_COLLIDER)
            .build()
    }
}

fn bounded(n: f32) -> bool {
    n.abs() <= MAX_COORD_M
}

#[derive(Deserialize)]
struct Document {
    version: u32,
    buildings: Vec<RawBuilding>,
}

#[derive(Deserialize)]
struct RawBuilding {
    id: String,
    walls: Vec<RawWall>,
    surfaces: Vec<RawSurface>,
}

#[derive(Deserialize)]
struct RawWall {
    a: [f32; 2],
    b: [f32; 2],
    bottom: f32,
    top: f32,
    width: f32,
}

#[derive(Deserialize)]
struct RawSurface {
    kind: SurfaceKind,
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum SurfaceKind {
    Floor,
    Roof,
    Ramp,
}
