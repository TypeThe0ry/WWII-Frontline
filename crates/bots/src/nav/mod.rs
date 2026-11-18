//! Navmesh navigation (M1-037): loads the `FNAV` file written by `map-pipeline nav` (M1-036),
//! finds paths over the Detour polygon graph and turns them into movement intents.
//!
//! Coordinates are the map's metres: `x` east, `z` north (Detour `y` is height and unused for
//! planning). Tile data is the standard Detour v7 layout; only polygons, vertices and neighbour
//! indices are read (links, detail meshes and the BV tree are not needed for A*).

pub mod avoid;
pub mod offmesh;
pub mod path;
pub mod steer;

use std::path::Path;

pub type V2 = [f32; 2];

const MAGIC: &[u8; 4] = b"FNAV";
const DT_MAGIC: i32 = 0x444E_4156;
const DT_VERSION: i32 = 7;
const HDR_BYTES: usize = 100;
const POLY_BYTES: usize = 32;
const EXT_LINK: u16 = 0x8000;
/// Marker for "no polygon".
pub const NO_POLY: u32 = u32::MAX;
const BIN_M: f32 = 16.0;
const EDGE_EPS: f32 = 1e-3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavError {
    Io(String),
    BadHeader,
    BadTile(String),
}

impl std::fmt::Display for NavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NavError::Io(e) => write!(f, "navmesh io: {e}"),
            NavError::BadHeader => write!(f, "navmesh: bad FNAV header"),
            NavError::BadTile(e) => write!(f, "navmesh: bad tile: {e}"),
        }
    }
}

impl std::error::Error for NavError {}

/// One walkable polygon (at most 6 vertices, planar in x/z).
#[derive(Debug, Clone)]
pub struct Poly {
    pub verts: Vec<V2>,
    pub y: f32,
    pub centre: V2,
    pub area: u8,
    /// Per edge `i` (vertex `i` to `i+1`): neighbour polygon and the shared portal segment.
    pub edges: Vec<Option<Portal>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Portal {
    pub to: u32,
    pub a: V2,
    pub b: V2,
}

#[derive(Debug, Clone)]
pub struct NavMesh {
    pub polys: Vec<Poly>,
    pub params_hash: [u8; 8],
    pub vehicle: bool,
    bins: Vec<Vec<u32>>,
    bx0: i32,
    bz0: i32,
    nbx: i32,
    nbz: i32,
}

fn rd_u32(b: &[u8], o: usize) -> Result<u32, NavError> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(NavError::BadHeader)
}
fn rd_i32(b: &[u8], o: usize) -> Result<i32, NavError> {
    rd_u32(b, o).map(|v| v as i32)
}
fn rd_f32(b: &[u8], o: usize) -> Result<f32, NavError> {
    rd_u32(b, o).map(f32::from_bits)
}
fn rd_u16(b: &[u8], o: usize) -> Result<u16, NavError> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(NavError::BadHeader)
}

struct RawPoly {
    verts: Vec<V2>,
    y: f32,
    neis: Vec<u16>,
    area: u8,
    ground: bool,
}

struct Tile {
    tx: i32,
    tz: i32,
    polys: Vec<RawPoly>,
}

fn parse_tile(tx: i32, tz: i32, d: &[u8]) -> Result<Tile, NavError> {
    let bad = |m: &str| NavError::BadTile(m.to_string());
    if d.len() < HDR_BYTES || rd_i32(d, 0)? != DT_MAGIC || rd_i32(d, 4)? != DT_VERSION {
        return Err(bad("detour header"));
    }
    let poly_count = rd_i32(d, 24)? as usize;
    let vert_count = rd_i32(d, 28)? as usize;
    let verts_off = HDR_BYTES;
    let polys_off = verts_off + vert_count * 12;
    if d.len() < polys_off + poly_count * POLY_BYTES {
        return Err(bad("truncated"));
    }
    let mut verts = Vec::with_capacity(vert_count);
    for i in 0..vert_count {
        let o = verts_off + i * 12;
        verts.push([rd_f32(d, o)?, rd_f32(d, o + 4)?, rd_f32(d, o + 8)?]);
    }
    let mut polys = Vec::with_capacity(poly_count);
    for i in 0..poly_count {
        let o = polys_off + i * POLY_BYTES;
        let vc = d[o + 30] as usize;
        let at = d[o + 31];
        if vc > 6 {
            return Err(bad("poly vertex count"));
        }
        let mut vs = Vec::with_capacity(vc);
        let mut ys = 0.0;
        for k in 0..vc {
            let vi = rd_u16(d, o + 4 + k * 2)? as usize;
            let v = verts.get(vi).ok_or_else(|| bad("vertex index"))?;
            vs.push([v[0], v[2]]);
            ys += v[1];
        }
        let neis = (0..vc)
            .map(|k| rd_u16(d, o + 16 + k * 2))
            .collect::<Result<Vec<_>, _>>()?;
        polys.push(RawPoly {
            verts: vs,
            y: if vc > 0 { ys / vc as f32 } else { 0.0 },
            neis,
            area: at & 0x3f,
            ground: at >> 6 == 0,
        });
    }
    Ok(Tile { tx, tz, polys })
}

fn centroid(v: &[V2]) -> V2 {
    let n = v.len().max(1) as f32;
    [
        v.iter().map(|p| p[0]).sum::<f32>() / n,
        v.iter().map(|p| p[1]).sum::<f32>() / n,
    ]
}

impl NavMesh {
    pub fn load(path: &Path) -> Result<Self, NavError> {
        let bytes = std::fs::read(path).map_err(|e| NavError::Io(e.to_string()))?;
        Self::from_fnav(&bytes)
    }

    pub fn from_fnav(b: &[u8]) -> Result<Self, NavError> {
        if b.len() < 36 || &b[0..4] != MAGIC || rd_u16(b, 4)? != 1 {
            return Err(NavError::BadHeader);
        }
        let vehicle = rd_u16(b, 6)? == 1;
        let mut params_hash = [0u8; 8];
        params_hash.copy_from_slice(&b[8..16]);
        let count = rd_u32(b, 16)? as usize;
        let mut o = 36;
        let mut tiles: Vec<Tile> = Vec::with_capacity(count);
        for _ in 0..count {
            let (tx, tz, len) = (rd_i32(b, o)?, rd_i32(b, o + 4)?, rd_u32(b, o + 8)? as usize);
            let data = b.get(o + 12..o + 12 + len).ok_or(NavError::BadHeader)?;
            let t = parse_tile(tx, tz, data)?;
            tiles.push(t);
            o += 12 + len;
        }
        tiles.sort_by_key(|t| (t.tx, t.tz));
        let mut index: std::collections::BTreeMap<(i32, i32), usize> =
            std::collections::BTreeMap::new();
        for (i, t) in tiles.iter().enumerate() {
            index.insert((t.tx, t.tz), i);
        }
        // global polygon ids follow the sorted tile order
        let mut gbase = vec![0u32; tiles.len()];
        let mut acc = 0u32;
        for (i, t) in tiles.iter().enumerate() {
            gbase[i] = acc;
            acc += t.polys.len() as u32;
        }
        let mut polys: Vec<Poly> = Vec::with_capacity(acc as usize);
        for (ti, t) in tiles.iter().enumerate() {
            for rp in &t.polys {
                let n = rp.verts.len();
                let mut edges: Vec<Option<Portal>> = vec![None; n];
                for (e, edge) in edges.iter_mut().enumerate() {
                    let nei = rp.neis[e];
                    let (a, bb) = (rp.verts[e], rp.verts[(e + 1) % n]);
                    if nei == 0 {
                        continue;
                    }
                    if nei & EXT_LINK == 0 {
                        let to = gbase[ti] + (nei as u32 - 1);
                        *edge = Some(Portal { to, a, b: bb });
                    } else {
                        let side = nei & 0xff;
                        let (dx, dz) = match side {
                            0 => (1, 0),
                            2 => (0, 1),
                            4 => (-1, 0),
                            6 => (0, -1),
                            _ => continue,
                        };
                        let Some(&nt) = index.get(&(t.tx + dx, t.tz + dz)) else {
                            continue;
                        };
                        let want = EXT_LINK | ((side + 4) & 7);
                        // overlap along the border axis (z for +-x borders, x for +-z borders)
                        let ax = if dx != 0 { 1 } else { 0 };
                        let (lo, hi) = (a[ax].min(bb[ax]), a[ax].max(bb[ax]));
                        for (pi, q) in tiles[nt].polys.iter().enumerate() {
                            if !q.ground {
                                continue;
                            }
                            let m = q.verts.len();
                            for f in 0..m {
                                if q.neis[f] != want {
                                    continue;
                                }
                                let (qa, qb) = (q.verts[f], q.verts[(f + 1) % m]);
                                let (qlo, qhi) = (qa[ax].min(qb[ax]), qa[ax].max(qb[ax]));
                                let (olo, ohi) = (lo.max(qlo), hi.min(qhi));
                                if ohi - olo > EDGE_EPS && edge.is_none() {
                                    let pt = |s: f32| {
                                        let t01 = if (bb[ax] - a[ax]).abs() < 1e-6 {
                                            0.0
                                        } else {
                                            (s - a[ax]) / (bb[ax] - a[ax])
                                        };
                                        [a[0] + (bb[0] - a[0]) * t01, a[1] + (bb[1] - a[1]) * t01]
                                    };
                                    *edge = Some(Portal {
                                        to: gbase[nt] + pi as u32,
                                        a: pt(olo),
                                        b: pt(ohi),
                                    });
                                }
                            }
                        }
                    }
                }
                polys.push(Poly {
                    verts: rp.verts.clone(),
                    y: rp.y,
                    centre: centroid(&rp.verts),
                    area: rp.area,
                    edges,
                });
            }
        }
        // off-mesh polygons (type != ground) are dropped from queries by clearing their edges
        let mut mesh = NavMesh {
            polys,
            params_hash,
            vehicle,
            bins: Vec::new(),
            bx0: 0,
            bz0: 0,
            nbx: 0,
            nbz: 0,
        };
        mesh.build_bins();
        Ok(mesh)
    }

    fn build_bins(&mut self) {
        let (mut x0, mut z0, mut x1, mut z1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in &self.polys {
            for v in &p.verts {
                x0 = x0.min(v[0]);
                z0 = z0.min(v[1]);
                x1 = x1.max(v[0]);
                z1 = z1.max(v[1]);
            }
        }
        if self.polys.is_empty() {
            return;
        }
        self.bx0 = (x0 / BIN_M).floor() as i32;
        self.bz0 = (z0 / BIN_M).floor() as i32;
        self.nbx = (x1 / BIN_M).floor() as i32 - self.bx0 + 1;
        self.nbz = (z1 / BIN_M).floor() as i32 - self.bz0 + 1;
        self.bins = vec![Vec::new(); (self.nbx * self.nbz) as usize];
        for (i, p) in self.polys.iter().enumerate() {
            let (mut a0, mut b0, mut a1, mut b1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for v in &p.verts {
                a0 = a0.min(v[0]);
                b0 = b0.min(v[1]);
                a1 = a1.max(v[0]);
                b1 = b1.max(v[1]);
            }
            for bz in (b0 / BIN_M).floor() as i32..=(b1 / BIN_M).floor() as i32 {
                for bx in (a0 / BIN_M).floor() as i32..=(a1 / BIN_M).floor() as i32 {
                    let (ix, iz) = (bx - self.bx0, bz - self.bz0);
                    if ix >= 0 && iz >= 0 && ix < self.nbx && iz < self.nbz {
                        self.bins[(iz * self.nbx + ix) as usize].push(i as u32);
                    }
                }
            }
        }
    }

    /// Builds a mesh from ready polygons (used by tests and tools).
    pub fn from_polys(polys: Vec<Poly>) -> Self {
        let mut m = NavMesh {
            polys,
            params_hash: [0; 8],
            vehicle: false,
            bins: Vec::new(),
            bx0: 0,
            bz0: 0,
            nbx: 0,
            nbz: 0,
        };
        m.build_bins();
        m
    }

    /// Loads the mesh or logs the reason and returns `None` so the caller keeps the M0 grid
    /// nav (M1-037 behaviour 1).
    pub fn load_or_log(path: &Path) -> Option<Self> {
        match Self::load(path) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!(
                    "bots: {e}; falling back to the M0 grid navigation ({})",
                    path.display()
                );
                None
            }
        }
    }

    pub fn poly_count(&self) -> usize {
        self.polys.len()
    }

    /// Nearest polygon within `tol` metres (point inside counts as distance 0); returns the
    /// polygon id and the projected point. Ties break to the lower id.
    pub fn nearest(&self, p: V2, tol: f32) -> Option<(u32, V2)> {
        if self.bins.is_empty() {
            return None;
        }
        let r = (tol / BIN_M).ceil() as i32;
        let (cx, cz) = (
            (p[0] / BIN_M).floor() as i32 - self.bx0,
            (p[1] / BIN_M).floor() as i32 - self.bz0,
        );
        let mut best: Option<(f32, u32, V2)> = None;
        for iz in (cz - r).max(0)..=(cz + r).min(self.nbz - 1) {
            for ix in (cx - r).max(0)..=(cx + r).min(self.nbx - 1) {
                for &id in &self.bins[(iz * self.nbx + ix) as usize] {
                    let poly = &self.polys[id as usize];
                    let (d, q) = if point_in_poly(p, &poly.verts) {
                        (0.0, p)
                    } else {
                        closest_on_poly(p, &poly.verts)
                    };
                    if d <= tol && best.is_none_or(|(bd, bi, _)| d < bd || (d == bd && id < bi)) {
                        best = Some((d, id, q));
                    }
                }
            }
        }
        best.map(|(_, i, q)| (i, q))
    }
}

pub fn point_in_poly(p: V2, v: &[V2]) -> bool {
    let mut inside = false;
    let mut j = v.len() - 1;
    for i in 0..v.len() {
        if (v[i][1] > p[1]) != (v[j][1] > p[1])
            && p[0] < (v[j][0] - v[i][0]) * (p[1] - v[i][1]) / (v[j][1] - v[i][1]) + v[i][0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn closest_on_seg(p: V2, a: V2, b: V2) -> V2 {
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let l = dx * dx + dz * dz;
    let t = if l == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dz) / l).clamp(0.0, 1.0)
    };
    [a[0] + t * dx, a[1] + t * dz]
}

fn closest_on_poly(p: V2, v: &[V2]) -> (f32, V2) {
    let mut best = (f32::MAX, p);
    for i in 0..v.len() {
        let q = closest_on_seg(p, v[i], v[(i + 1) % v.len()]);
        let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt();
        if d < best.0 {
            best = (d, q);
        }
    }
    best
}
