//! One 64 m terrain chunk and the parser for the `c_{bx}_{bz}.bin` format of M0-009.
//!
//! ```text
//! "FCOL" | u8 version=1 | u8 pad | u16 samples=65 | f32 hmin | f32 hmax   (16 B, little endian)
//! u16 heights[65*65]   (row = z, column = x)   h = hmin + u16 / 65535 * (hmax - hmin)
//! u8  material[64*64]  (row = z, column = x, per cell)
//! ```

use super::{CHUNK_CELLS, CHUNK_SAMPLES};
use crate::hash::HashWriter;

const MAGIC: &[u8; 4] = b"FCOL";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 16;
const N_H: usize = CHUNK_SAMPLES * CHUNK_SAMPLES;
const N_M: usize = CHUNK_CELLS * CHUNK_CELLS;
/// Exact byte length of a valid file.
pub const CHUNK_FILE_LEN: usize = HEADER_LEN + N_H * 2 + N_M;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionError {
    /// Wrong byte length (truncated or oversized).
    BadLength { expected: usize, got: usize },
    BadMagic,
    BadVersion(u8),
    /// `samples` field is not 65.
    BadSamples(u16),
    /// `hmin`/`hmax` not finite or `hmin > hmax`.
    BadRange,
}

impl std::fmt::Display for CollisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadLength { expected, got } => {
                write!(f, "collision chunk length {got}, expected {expected}")
            }
            Self::BadMagic => write!(f, "collision chunk: bad magic"),
            Self::BadVersion(v) => write!(f, "collision chunk: unsupported version {v}"),
            Self::BadSamples(s) => write!(f, "collision chunk: samples {s}, expected 65"),
            Self::BadRange => write!(f, "collision chunk: invalid height range"),
        }
    }
}

impl std::error::Error for CollisionError {}

/// Dequantised chunk. `heights_m[z * 65 + x]` (metres), `material_ids[z * 64 + x]`.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionChunk {
    pub bx: i32,
    pub bz: i32,
    pub heights_m: Vec<f32>,
    pub material_ids: Vec<u8>,
    min_h: f32,
    max_h: f32,
}

impl CollisionChunk {
    /// Builds from already-dequantised samples (tests, synthetic terrain).
    ///
    /// # Panics
    /// If the slices have the wrong length (programmer error; use [`Self::from_bytes`] for
    /// untrusted input).
    pub fn new(bx: i32, bz: i32, heights_m: Vec<f32>, material_ids: Vec<u8>) -> Self {
        assert_eq!(heights_m.len(), N_H, "heights_m must hold 65*65 samples");
        assert_eq!(material_ids.len(), N_M, "material_ids must hold 64*64 cells");
        let min_h = heights_m.iter().copied().fold(f32::INFINITY, f32::min);
        let max_h = heights_m.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        Self {
            bx,
            bz,
            heights_m,
            material_ids,
            min_h,
            max_h,
        }
    }

    /// Parses one `c_*.bin`. Never panics and allocates only after the length and header
    /// have been validated (at most ~25 KB).
    pub fn from_bytes(bx: i32, bz: i32, bytes: &[u8]) -> Result<Self, CollisionError> {
        if bytes.len() != CHUNK_FILE_LEN {
            // Check magic first when we can, so a wrong file type reports BadMagic.
            if bytes.len() >= 4 && &bytes[0..4] != MAGIC {
                return Err(CollisionError::BadMagic);
            }
            return Err(CollisionError::BadLength {
                expected: CHUNK_FILE_LEN,
                got: bytes.len(),
            });
        }
        if &bytes[0..4] != MAGIC {
            return Err(CollisionError::BadMagic);
        }
        if bytes[4] != VERSION {
            return Err(CollisionError::BadVersion(bytes[4]));
        }
        let samples = u16::from_le_bytes([bytes[6], bytes[7]]);
        if samples as usize != CHUNK_SAMPLES {
            return Err(CollisionError::BadSamples(samples));
        }
        let hmin = f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let hmax = f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        if !hmin.is_finite() || !hmax.is_finite() || hmin > hmax {
            return Err(CollisionError::BadRange);
        }
        let span = hmax - hmin;
        let hbytes = &bytes[HEADER_LEN..HEADER_LEN + N_H * 2];
        let heights_m: Vec<f32> = hbytes
            .chunks_exact(2)
            .map(|p| {
                let q = u16::from_le_bytes([p[0], p[1]]);
                hmin + (q as f32) / 65535.0 * span
            })
            .collect();
        let material_ids = bytes[HEADER_LEN + N_H * 2..].to_vec();
        Ok(Self::new(bx, bz, heights_m, material_ids))
    }

    /// Sample height at integer sample coordinates (0..=64).
    #[inline]
    pub fn sample(&self, ix: usize, iz: usize) -> f32 {
        self.heights_m[iz * CHUNK_SAMPLES + ix]
    }

    /// Material id of cell `(cx, cz)` (0..64).
    #[inline]
    pub fn material(&self, cx: usize, cz: usize) -> u8 {
        self.material_ids[cz * CHUNK_CELLS + cx]
    }

    pub fn min_height(&self) -> f32 {
        self.min_h
    }

    pub fn max_height(&self) -> f32 {
        self.max_h
    }

    pub fn memory_bytes(&self) -> usize {
        self.heights_m.len() * 4 + self.material_ids.len()
    }

    pub(crate) fn hash_into(&self, h: &mut HashWriter) {
        h.write_u32(self.bx as u32);
        h.write_u32(self.bz as u32);
        for &v in &self.heights_m {
            h.write_f32(v);
        }
        h.write_bytes(&self.material_ids);
    }
}
