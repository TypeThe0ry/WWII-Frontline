//! Parser robustness for `c_*.bin` (`cargo test collision_parse_fuzz`).
mod common;

use common::Rng;
use frontline_sim::{CollisionChunk, CollisionError};

const LEN: usize = 16 + 65 * 65 * 2 + 64 * 64;

fn valid(rng: &mut Rng) -> Vec<u8> {
    let mut b = Vec::with_capacity(LEN);
    b.extend_from_slice(b"FCOL");
    b.extend_from_slice(&[1, 0]);
    b.extend_from_slice(&65u16.to_le_bytes());
    b.extend_from_slice(&(-3.5f32).to_le_bytes());
    b.extend_from_slice(&40.25f32.to_le_bytes());
    while b.len() < LEN {
        b.push((rng.next_u64() >> 24) as u8);
    }
    b
}

#[test]
fn collision_parse_valid_and_named_errors() {
    let mut rng = Rng(1);
    let ok = valid(&mut rng);
    let c = CollisionChunk::from_bytes(-2, 7, &ok).unwrap();
    assert_eq!((c.bx, c.bz), (-2, 7));
    assert_eq!(c.heights_m.len(), 65 * 65);
    assert!(c.min_height() >= -3.5 - 1e-4 && c.max_height() <= 40.25 + 1e-4);
    // Dequantisation of the extreme codes.
    let mut b = ok.clone();
    b[16..20].copy_from_slice(&[0, 0, 0xff, 0xff]);
    let c = CollisionChunk::from_bytes(0, 0, &b).unwrap();
    assert_eq!(c.heights_m[0], -3.5);
    assert_eq!(c.heights_m[1], 40.25);

    assert!(matches!(
        CollisionChunk::from_bytes(0, 0, &ok[..LEN - 1]),
        Err(CollisionError::BadLength { .. })
    ));
    assert!(matches!(
        CollisionChunk::from_bytes(0, 0, &[]),
        Err(CollisionError::BadLength { .. })
    ));
    let mut long = ok.clone();
    long.push(0);
    assert!(matches!(
        CollisionChunk::from_bytes(0, 0, &long),
        Err(CollisionError::BadLength { .. })
    ));
    let mut b = ok.clone();
    b[0] = b'X';
    assert_eq!(
        CollisionChunk::from_bytes(0, 0, &b),
        Err(CollisionError::BadMagic)
    );
    let mut b = ok.clone();
    b[4] = 2;
    assert_eq!(
        CollisionChunk::from_bytes(0, 0, &b),
        Err(CollisionError::BadVersion(2))
    );
    let mut b = ok.clone();
    b[6..8].copy_from_slice(&0xffffu16.to_le_bytes());
    assert_eq!(
        CollisionChunk::from_bytes(0, 0, &b),
        Err(CollisionError::BadSamples(0xffff))
    );
    let mut b = ok.clone();
    b[8..12].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(
        CollisionChunk::from_bytes(0, 0, &b),
        Err(CollisionError::BadRange)
    );
    let mut b = ok.clone();
    b[8..12].copy_from_slice(&50.0f32.to_le_bytes());
    assert_eq!(
        CollisionChunk::from_bytes(0, 0, &b),
        Err(CollisionError::BadRange)
    );
}

/// 10^5 random inputs: truncations, header mutations, byte flips and pure noise must give `Ok`
/// (only for fully valid files) or `Err`, never a panic. A valid parse holds at most ~21 KB
/// (`memory_bytes`), far below the 1 MB allocation bound; rejected inputs allocate nothing
/// because every check precedes the first `Vec`.
#[test]
fn collision_parse_fuzz() {
    let mut rng = Rng(0xdead_beef_1234);
    let base = valid(&mut rng);
    let (mut oks, mut errs) = (0u32, 0u32);
    for i in 0..100_000u32 {
        let mut b = match i % 4 {
            0 => base[..((rng.next_u64() as usize) % (LEN + 64)).min(LEN)].to_vec(),
            1 => {
                let mut b = base.clone();
                for _ in 0..1 + rng.next_u64() % 8 {
                    let k = (rng.next_u64() as usize) % 24; // header region
                    b[k] = (rng.next_u64() >> 17) as u8;
                }
                b
            }
            2 => {
                let mut b = base.clone();
                for _ in 0..1 + rng.next_u64() % 8 {
                    let k = (rng.next_u64() as usize) % LEN;
                    b[k] ^= 1 << (rng.next_u64() % 8);
                }
                b
            }
            _ => (0..(rng.next_u64() as usize) % 300)
                .map(|_| (rng.next_u64() >> 11) as u8)
                .collect(),
        };
        if i % 4 == 3 && b.len() > 4 && i % 8 == 3 {
            b[..4].copy_from_slice(b"FCOL");
        }
        match CollisionChunk::from_bytes((i % 5) as i32 - 2, 0, &b) {
            Ok(c) => {
                oks += 1;
                assert_eq!(b.len(), LEN);
                assert!(c.memory_bytes() < 1 << 20);
                assert!(c.heights_m.iter().all(|h| h.is_finite()));
            }
            Err(_) => errs += 1,
        }
    }
    println!("fuzz: {oks} ok / {errs} err");
    assert_eq!(oks + errs, 100_000);
}
