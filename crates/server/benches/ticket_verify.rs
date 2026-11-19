//! PI-076 verification budget. Run `cargo bench -p frontline-server --bench ticket_verify`.

use std::hint::black_box;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use criterion::{criterion_group, criterion_main, Criterion};
use ed25519_dalek::{Signer, SigningKey};
use frontline_protocol::dto::{TicketClaims, TICKET_DOMAIN};
use frontline_server::gateway_link::ticket_verify::{TicketVerifier, JTI_CAPACITY};

fn verify(c: &mut Criterion) {
    let signing = SigningKey::from_bytes(&[5; 32]);
    let match_id = [1; 16];
    let key = signing.verifying_key();
    let other_key = SigningKey::from_bytes(&[6; 32]).verifying_key();
    // Rotate through one more JTI than the cache holds. Each iteration measures successful
    // admission (signature, claims and replay-cache insertion) with a full cache.
    let tickets: Vec<_> = (0..=JTI_CAPACITY)
        .map(|i| {
            let claims = TicketClaims {
                player_id: 0,
                match_id,
                nick: "Fixture".into(),
                exp: 60,
                jti: (i as u128).to_le_bytes(),
                is_admin: false,
            };
            let bytes = claims.encode();
            let signature = signing.sign(&[TICKET_DOMAIN, bytes.as_slice()].concat());
            format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(bytes),
                URL_SAFE_NO_PAD.encode(signature.to_bytes())
            )
        })
        .collect();
    for (name, keys) in [
        ("single_key", vec![key]),
        ("second_key", vec![other_key, key]),
    ] {
        let mut verifier = TicketVerifier::new(keys);
        for raw in &tickets[..JTI_CAPACITY] {
            verifier.verify(raw.as_bytes(), 1, match_id).unwrap();
        }
        let mut i = JTI_CAPACITY;
        c.bench_function(name, |b| {
            b.iter(|| {
                let raw = tickets[i].as_bytes();
                i = (i + 1) % tickets.len();
                black_box(verifier.verify(black_box(raw), 1, match_id).unwrap());
            })
        });
    }
}

criterion_group!(benches, verify);
criterion_main!(benches);
