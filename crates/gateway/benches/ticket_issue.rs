use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ed25519_dalek::SigningKey;
use frontline_gateway::auth::ticket;
use frontline_protocol::dto::TicketClaims;
use rand::RngCore;

fn benchmark(c: &mut Criterion) {
    let key = SigningKey::from_bytes(&[7; 32]);
    c.bench_function("ticket_issue", |b| {
        b.iter(|| {
            let mut jti = [0; 16];
            rand::rngs::OsRng.fill_bytes(&mut jti);
            ticket::issue(
                black_box(&key),
                TicketClaims {
                    player_id: 7,
                    match_id: [8; 16],
                    nick: "Guest-1234".into(),
                    exp: 0,
                    jti,
                    is_admin: false,
                },
                1_800_000_000,
            )
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
