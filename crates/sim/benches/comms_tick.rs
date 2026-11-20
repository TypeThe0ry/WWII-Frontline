//! M2-019 registry budget. Exercises both 200 markers / 100 players and 100 / 200.
//! Run `cargo bench -p frontline-sim --bench comms_tick`.
use std::hint::black_box;
use std::time::{Duration, Instant};

use frontline_sim::comms::{Marker, MarkerKind, MarkerRegistry, Member};
use frontline_sim::{GameData, Team, Tick, Vec3};

const HZ: u32 = 30;
const SAMPLES: usize = 41;
const STEADY_TICKS: u32 = 120;
const DUE: usize = 12;

fn fixture(players: u8, markers: usize) -> (MarkerRegistry, Vec<Member>) {
    use MarkerKind::*;

    let mut registry = MarkerRegistry::new(GameData::comms(), HZ);
    let viewers: Vec<_> = (0..players)
        .map(|player| {
            let lead = player % 5 == 0;
            let role =
                ["officer", "medic", "support", "engineer", "rifleman"][usize::from(player % 5)];
            Member {
                player,
                team: if player < players / 2 {
                    Team::Us
                } else {
                    Team::Ger
                },
                squad: Some(u16::from(player / 5)),
                role: Some(role),
                lead,
                joined: Tick(0),
            }
        })
        .collect();
    // Twelve authoritative enemy markers are due for observation and publication together.
    let mut marks = Vec::with_capacity(markers);
    for &author in viewers.iter().filter(|author| author.lead).take(DUE) {
        marks.push((author, TeamVehicle));
    }
    for &author in viewers.iter().step_by(2).take(markers - DUE) {
        marks.push((author, Enemy));
    }
    for &author in viewers
        .iter()
        .skip(1)
        .step_by(2)
        .take(markers - marks.len())
    {
        marks.push((author, NeedMedic));
    }
    for &author in viewers.iter().filter(|author| author.lead) {
        for kind in [OrderMove, OrderAttack, OrderDefend, OrderHold, OrderBuild] {
            if marks.len() < markers {
                marks.push((author, kind));
            }
        }
    }
    assert_eq!(marks.len(), markers);
    for (i, (author, kind)) in marks.into_iter().enumerate() {
        registry
            .place(author, kind, Vec3::ZERO, Some(i as u16), Tick(0))
            .unwrap();
    }
    (registry, viewers)
}

// Mirror the server's audience bitset with enough words for the 200-viewer stress case.
fn audience(registry: &MarkerRegistry, viewers: &[Member], marker: &Marker, now: Tick) -> [u64; 4] {
    let mut bits = [0; 4];
    for &viewer in viewers {
        if registry.visible_to(marker, viewer, now) {
            let slot = usize::from(viewer.player);
            bits[slot / 64] |= 1 << (slot % 64);
        }
    }
    bits
}

fn visit_recipients(bits: [u64; 4]) {
    for (word, mut bits) in bits.into_iter().enumerate() {
        while bits != 0 {
            black_box(word * 64 + bits.trailing_zeros() as usize);
            bits &= bits - 1;
        }
    }
}

fn us(sample: Duration, n: u32) -> f64 {
    sample.as_secs_f64() * 1e6 / f64::from(n)
}

fn measure(players: u8, markers: usize) {
    let mut steady = [0.0; SAMPLES];
    let mut exhaustive = [0.0; SAMPLES];
    let mut due = [0.0; SAMPLES];
    let mut expire = [0.0; SAMPLES];
    for sample in 0..SAMPLES {
        let (mut registry, viewers) = fixture(players, markers);
        let initial = registry.take_due(Tick(0));
        assert_eq!(initial.len(), markers);
        let mut cached = vec![[0; 4]; markers + 1];
        for marker in &initial {
            cached[marker.id as usize] = audience(&registry, &viewers, marker, Tick(0));
        }
        let start = Instant::now();
        for tick in 1..=STEADY_TICKS {
            let now = black_box(Tick(tick));
            black_box(registry.tick(now));
            black_box(registry.take_due(now));
        }
        steady[sample] = us(start.elapsed(), STEADY_TICKS);

        let start = Instant::now();
        for _ in 0..STEADY_TICKS {
            let now = black_box(Tick(STEADY_TICKS));
            for &viewer in &viewers {
                black_box(registry.visible(viewer, now).count());
            }
        }
        exhaustive[sample] = us(start.elapsed(), STEADY_TICKS);

        let now = Tick(STEADY_TICKS + 1);
        let moved = initial.iter().filter(|m| m.kind == MarkerKind::TeamVehicle);
        let start = Instant::now();
        black_box(registry.tick(now));
        for marker in moved {
            registry
                .move_marker(marker.author, marker.id, Vec3::splat(10.0), now)
                .unwrap();
        }
        let updates = registry.take_due(now);
        for marker in &updates {
            let bits = audience(&registry, &viewers, marker, now);
            cached[marker.id as usize] = bits;
            visit_recipients(bits);
        }
        due[sample] = us(start.elapsed(), 1);
        assert_eq!(updates.len(), DUE);

        let now = Tick(6 * HZ);
        let start = Instant::now();
        let expired = registry.tick(now);
        for marker in &expired {
            visit_recipients(std::mem::take(&mut cached[marker.id as usize]));
        }
        expire[sample] = us(start.elapsed(), 1);
        assert_eq!(expired.len(), usize::from(players / 2).min(markers - DUE));
    }
    for (name, samples) in [
        ("steady queue scans", &mut steady),
        ("exhaustive visibility scan", &mut exhaustive),
        ("12 moves/due/fan-out", &mut due),
        ("enemy expiry/cached fan-out", &mut expire),
    ] {
        samples.sort_by(f64::total_cmp);
        println!("comms_tick {players:>3} viewers / {markers:>3} markers, {name:<29} p50 {:>6.2} us, p95 {:>6.2} us", samples[SAMPLES / 2], samples[SAMPLES * 95 / 100]);
    }
}

fn main() {
    measure(100, 200);
    measure(200, 100);
}
