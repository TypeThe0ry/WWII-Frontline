//! State hashing: FNV-1a 64 over explicitly written fields (no `std::hash`).
//!
//! Float convention: `f32` is written as its IEEE-754 bit pattern after canonicalisation:
//! every NaN becomes `0x7fc0_0000` and `-0.0` becomes `+0.0`. All integers are written
//! little-endian with their natural width.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone)]
pub struct HashWriter(u64);

impl Default for HashWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl HashWriter {
    pub fn new() -> Self {
        HashWriter(FNV_OFFSET)
    }
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(FNV_PRIME);
        }
    }
    pub fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }
    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }
    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }
    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }
    pub fn write_i8(&mut self, v: i8) {
        self.write_u8(v as u8);
    }
    pub fn write_i16(&mut self, v: i16) {
        self.write_u16(v as u16);
    }
    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(canonical_f32_bits(v));
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// Bit pattern used for hashing (see module docs).
#[inline]
pub fn canonical_f32_bits(v: f32) -> u32 {
    if v.is_nan() {
        0x7fc0_0000
    } else if v == 0.0 {
        0
    } else {
        v.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_vectors() {
        let mut h = HashWriter::new();
        assert_eq!(h.finish(), 0xcbf2_9ce4_8422_2325);
        h.write_bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        let mut h = HashWriter::new();
        h.write_bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn negative_zero_and_nan_are_canonical() {
        assert_eq!(canonical_f32_bits(-0.0), canonical_f32_bits(0.0));
        let nan2 = f32::from_bits(0x7fc0_1234);
        assert_eq!(canonical_f32_bits(nan2), canonical_f32_bits(f32::NAN));
        assert_eq!(canonical_f32_bits(-f32::NAN), 0x7fc0_0000);
        assert_ne!(canonical_f32_bits(1.0), canonical_f32_bits(-1.0));
        let (mut a, mut b) = (HashWriter::new(), HashWriter::new());
        a.write_f32(-0.0);
        b.write_f32(0.0);
        assert_eq!(a.finish(), b.finish());
    }
}
