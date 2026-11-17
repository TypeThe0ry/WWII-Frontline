//! MSB-first bit writer and reader. The reader never panics: out-of-range reads return
//! [`CodecError::Eof`].

use alloc::vec::Vec;

use crate::codec::CodecError;

/// Appends bit fields to a growing byte buffer (MSB first; last byte zero padded).
#[derive(Debug, Default, Clone)]
pub struct BitWriter {
    buf: Vec<u8>,
    /// Number of bits written.
    bits: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            buf: Vec::with_capacity(bytes),
            bits: 0,
        }
    }

    /// Bits written so far.
    pub fn bit_len(&self) -> usize {
        self.bits
    }

    /// Write the low `n` bits of `v` (`n <= 64`). `Overflow` if `v` needs more than `n` bits
    /// or `n > 64`.
    pub fn write_bits(&mut self, v: u64, n: u8) -> Result<(), CodecError> {
        if n > 64 {
            return Err(CodecError::Overflow);
        }
        if n < 64 && (v >> n) != 0 {
            return Err(CodecError::Overflow);
        }
        let mut left = n as usize;
        while left > 0 {
            let used = self.bits % 8;
            if used == 0 {
                self.buf.push(0);
            }
            let free = 8 - used;
            let take = free.min(left);
            // top `take` bits of the remaining `left` bits of v
            let chunk = ((v >> (left - take)) & ((1u64 << take) - 1)) as u8;
            let last = self.buf.len() - 1;
            self.buf[last] |= chunk << (free - take);
            self.bits += take;
            left -= take;
        }
        Ok(())
    }

    pub fn write_bool(&mut self, b: bool) -> Result<(), CodecError> {
        self.write_bits(b as u64, 1)
    }

    /// Write `v` as an `n`-bit two's-complement integer (`1 <= n <= 64`).
    pub fn write_i(&mut self, v: i64, n: u8) -> Result<(), CodecError> {
        if n == 0 || n > 64 {
            return Err(CodecError::Overflow);
        }
        if n < 64 {
            let min = -(1i64 << (n - 1));
            let max = (1i64 << (n - 1)) - 1;
            if v < min || v > max {
                return Err(CodecError::Overflow);
            }
        }
        let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
        self.write_bits((v as u64) & mask, n)
    }

    /// Write `bytes.len()` in `len_bits` bits, then the bytes (8 bits each, not aligned).
    pub fn write_bytes_len_prefixed(
        &mut self,
        bytes: &[u8],
        len_bits: u8,
    ) -> Result<(), CodecError> {
        self.write_bits(bytes.len() as u64, len_bits)
            .map_err(|_| CodecError::TooLong)?;
        for &b in bytes {
            self.write_bits(b as u64, 8)?;
        }
        Ok(())
    }

    /// Finish and return the bytes (zero padded to a byte boundary).
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

/// Reads bit fields written by [`BitWriter`].
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Bits still unread.
    pub fn remaining_bits(&self) -> usize {
        self.data.len() * 8 - self.pos
    }

    pub fn read_bits(&mut self, n: u8) -> Result<u64, CodecError> {
        if n > 64 {
            return Err(CodecError::Overflow);
        }
        if (n as usize) > self.remaining_bits() {
            return Err(CodecError::Eof);
        }
        let mut left = n as usize;
        let mut out = 0u64;
        while left > 0 {
            let byte = self.data[self.pos / 8];
            let used = self.pos % 8;
            let avail = 8 - used;
            let take = avail.min(left);
            let chunk = (byte >> (avail - take)) & (((1u16 << take) - 1) as u8);
            out = (out << take) | chunk as u64;
            self.pos += take;
            left -= take;
        }
        Ok(out)
    }

    pub fn read_bool(&mut self) -> Result<bool, CodecError> {
        Ok(self.read_bits(1)? == 1)
    }

    /// Read an `n`-bit two's-complement integer (`1 <= n <= 64`), sign extended.
    pub fn read_i(&mut self, n: u8) -> Result<i64, CodecError> {
        if n == 0 || n > 64 {
            return Err(CodecError::Overflow);
        }
        let raw = self.read_bits(n)?;
        if n == 64 {
            return Ok(raw as i64);
        }
        let sign = 1u64 << (n - 1);
        Ok(if raw & sign != 0 {
            (raw | !((1u64 << n) - 1)) as i64
        } else {
            raw as i64
        })
    }

    /// Read a length prefix of `len_bits` bits and that many bytes. `TooLong` if the length
    /// exceeds `max_len` or the remaining input (checked **before** allocating).
    pub fn read_bytes_len_prefixed(
        &mut self,
        len_bits: u8,
        max_len: usize,
    ) -> Result<Vec<u8>, CodecError> {
        let len = self.read_bits(len_bits)? as usize;
        if len > max_len || len.saturating_mul(8) > self.remaining_bits() {
            return Err(CodecError::TooLong);
        }
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(self.read_bits(8)? as u8);
        }
        Ok(v)
    }

    /// Succeed if only zero padding (< 8 bits) is left; otherwise `Trailing`.
    pub fn finish(&mut self) -> Result<(), CodecError> {
        let rem = self.remaining_bits();
        if rem >= 8 {
            return Err(CodecError::Trailing);
        }
        if rem > 0 && self.read_bits(rem as u8)? != 0 {
            return Err(CodecError::Trailing);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_bits_and_cross_byte() {
        let mut w = BitWriter::new();
        w.write_bits(0, 0).unwrap();
        w.write_bits(0b101, 3).unwrap();
        w.write_bits(0x1FF, 9).unwrap();
        w.write_bits(u64::MAX, 64).unwrap();
        w.write_bits(0x2A, 6).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(0).unwrap(), 0);
        assert_eq!(r.read_bits(3).unwrap(), 0b101);
        assert_eq!(r.read_bits(9).unwrap(), 0x1FF);
        assert_eq!(r.read_bits(64).unwrap(), u64::MAX);
        assert_eq!(r.read_bits(6).unwrap(), 0x2A);
        r.finish().unwrap();
    }

    #[test]
    fn overflow_detected() {
        let mut w = BitWriter::new();
        assert_eq!(w.write_bits(4, 2), Err(CodecError::Overflow));
        assert_eq!(w.write_bits(0, 65), Err(CodecError::Overflow));
        assert_eq!(w.write_i(8, 4), Err(CodecError::Overflow));
        assert_eq!(w.write_i(-9, 4), Err(CodecError::Overflow));
    }

    #[test]
    fn twos_complement_sign_extension() {
        for n in 1u8..=64 {
            let (min, max) = if n == 64 {
                (i64::MIN, i64::MAX)
            } else {
                (-(1i64 << (n - 1)), (1i64 << (n - 1)) - 1)
            };
            for v in [min, max, 0, -1.max(min), 1.min(max)] {
                let mut w = BitWriter::new();
                w.write_i(v, n).unwrap();
                let bytes = w.finish();
                let mut r = BitReader::new(&bytes);
                assert_eq!(r.read_i(n).unwrap(), v, "n={n}");
            }
        }
    }

    #[test]
    fn eof_not_panic() {
        let mut r = BitReader::new(&[0xFF]);
        assert_eq!(r.read_bits(9), Err(CodecError::Eof));
        assert_eq!(r.read_bits(8).unwrap(), 0xFF);
        assert_eq!(r.read_bits(1), Err(CodecError::Eof));
        assert_eq!(r.read_bits(0).unwrap(), 0);
        assert_eq!(r.read_bits(65), Err(CodecError::Overflow));
    }

    #[test]
    fn bytes_len_prefixed_limits() {
        let mut w = BitWriter::new();
        w.write_bytes_len_prefixed(&[1, 2, 3], 9).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bytes_len_prefixed(9, 256).unwrap(), [1, 2, 3]);
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bytes_len_prefixed(9, 2), Err(CodecError::TooLong));
        // claims 511 bytes but input is tiny: rejected without allocating
        let mut r = BitReader::new(&[0xFF, 0xFF]);
        assert_eq!(r.read_bytes_len_prefixed(9, 1000), Err(CodecError::TooLong));
    }

    #[test]
    fn finish_rejects_trailing() {
        let mut r = BitReader::new(&[0, 0]);
        assert_eq!(r.finish(), Err(CodecError::Trailing));
        let mut r = BitReader::new(&[0b0000_0001]);
        r.read_bits(4).unwrap();
        assert_eq!(r.finish(), Err(CodecError::Trailing));
    }
}
