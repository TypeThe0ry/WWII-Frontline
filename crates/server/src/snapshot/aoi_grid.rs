//! 64 m uniform grid over the players (ARCHITECTURE §6.5.4).
//!
//! `cell = floor(pos.xz / 64)`. [`AoiGrid::update`] is called for every entity every tick with
//! its exact position and only touches the cell lists when the entity crossed a cell border.
//! Because the grid always holds the *current* positions, the 3x3 neighbourhood plus a
//! velocity-extrapolation margin that the §5.7 risk note worries about is not needed here: the
//! query box is simply `ceil(radius / 64)` cells around the query cell, which contains every
//! entity within `radius` of any point of that cell. (A sprinting soldier covers 0.2 m per
//! tick, so a stale cell would only matter if the grid were updated less than every tick.)

use std::collections::BTreeMap;

/// Cell edge in metres.
pub const CELL_M: f32 = 64.0;
/// Cells per axis (2048 m world, the 16-bit position range).
pub const CELLS_PER_AXIS: i32 = 32;
/// Near field: 5x5 cells around the client's cell.
pub const NEAR_RADIUS_CELLS: i32 = 2;

#[derive(Debug, Default)]
pub struct AoiGrid {
    cells: BTreeMap<(i32, i32), Vec<u8>>,
    loc: BTreeMap<u8, (i32, i32)>,
}

/// Cell of a world position (clamped to the grid).
pub fn cell_of(x: f32, z: f32) -> (i32, i32) {
    let f = |v: f32| ((v / CELL_M).floor() as i32).clamp(0, CELLS_PER_AXIS - 1);
    (f(x), f(z))
}

impl AoiGrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.loc.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loc.is_empty()
    }

    /// Inserts or moves `id`; returns `true` when its cell changed (or it was new).
    pub fn update(&mut self, id: u8, x: f32, z: f32) -> bool {
        let c = cell_of(x, z);
        match self.loc.get(&id).copied() {
            Some(old) if old == c => false,
            Some(old) => {
                self.take_from(old, id);
                self.cells.entry(c).or_default().push(id);
                self.loc.insert(id, c);
                true
            }
            None => {
                self.cells.entry(c).or_default().push(id);
                self.loc.insert(id, c);
                true
            }
        }
    }

    fn take_from(&mut self, c: (i32, i32), id: u8) {
        if let Some(v) = self.cells.get_mut(&c) {
            if let Some(i) = v.iter().position(|&e| e == id) {
                v.swap_remove(i);
            }
            if v.is_empty() {
                self.cells.remove(&c);
            }
        }
    }

    pub fn remove(&mut self, id: u8) {
        if let Some(c) = self.loc.remove(&id) {
            self.take_from(c, id);
        }
    }

    pub fn cell(&self, id: u8) -> Option<(i32, i32)> {
        self.loc.get(&id).copied()
    }

    /// Appends every entity in the `(2r+1)^2` cells around `cell` to `out`.
    pub fn query(&self, cell: (i32, i32), radius_cells: i32, out: &mut Vec<u8>) {
        for cx in (cell.0 - radius_cells)..=(cell.0 + radius_cells) {
            for cz in (cell.1 - radius_cells)..=(cell.1 + radius_cells) {
                if let Some(v) = self.cells.get(&(cx, cz)) {
                    out.extend_from_slice(v);
                }
            }
        }
    }

    /// Everything that may lie within `radius_m` of `(x, z)`: the near field (5x5 cells) and,
    /// for larger radii, the far field out to `ceil(radius / 64)` cells. Callers apply the exact
    /// distance test.
    pub fn query_radius(&self, x: f32, z: f32, radius_m: f32, out: &mut Vec<u8>) {
        let r = ((radius_m / CELL_M).ceil() as i32).max(NEAR_RADIUS_CELLS);
        self.query(cell_of(x, z), r, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_only_moves_on_cell_crossing() {
        let mut g = AoiGrid::new();
        assert!(g.update(1, 10.0, 10.0));
        assert!(!g.update(1, 63.9, 0.1));
        assert_eq!(g.cell(1), Some((0, 0)));
        assert!(g.update(1, 64.0, 0.1));
        assert_eq!(g.cell(1), Some((1, 0)));
        assert_eq!(g.len(), 1);
        let mut v = vec![];
        g.query((0, 0), 0, &mut v);
        assert!(v.is_empty());
        g.query((1, 0), 0, &mut v);
        assert_eq!(v, vec![1]);
    }

    #[test]
    fn five_by_five_neighbourhood() {
        let mut g = AoiGrid::new();
        // client in cell (10, 10); entities at offsets -3..=3 cells in x
        for (i, dx) in (-3..=3).enumerate() {
            g.update(
                i as u8,
                (10 + dx) as f32 * CELL_M + 1.0,
                10.0 * CELL_M + 1.0,
            );
        }
        let mut v = vec![];
        g.query((10, 10), NEAR_RADIUS_CELLS, &mut v);
        v.sort();
        assert_eq!(v, vec![1, 2, 3, 4, 5]); // dx = -2..=2
    }

    #[test]
    fn radius_query_covers_everything_within_the_radius() {
        let mut g = AoiGrid::new();
        let (cx, cz) = (900.0f32, 900.0f32);
        let mut want = vec![];
        for i in 0..100u8 {
            let a = i as f32 * 0.37;
            let r = 20.0 + i as f32 * 4.3;
            let (x, z) = (cx + a.cos() * r, cz + a.sin() * r);
            g.update(i, x, z);
            if r <= 400.0 {
                want.push(i);
            }
        }
        let mut got = vec![];
        g.query_radius(cx, cz, 400.0, &mut got);
        for w in want {
            assert!(got.contains(&w), "entity {w} within 400 m missing");
        }
    }

    #[test]
    fn remove_clears_membership() {
        let mut g = AoiGrid::new();
        g.update(7, 5.0, 5.0);
        g.remove(7);
        g.remove(7);
        assert!(g.is_empty());
        let mut v = vec![];
        g.query((0, 0), 3, &mut v);
        assert!(v.is_empty());
    }
}
