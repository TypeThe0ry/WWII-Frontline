//! Worst production-limit mine count against 100 player and 20 vehicle contacts.
use std::hint::black_box;
use std::time::Instant;

use frontline_sim::mine::{InfantryContact, MineKind, Minefield, VehicleContact};
use frontline_sim::Tick;
use frontline_sim::{Team, Vec3};

fn main() {
    let mut mines = Minefield::default();
    for i in 0..20u8 {
        mines
            .place(
                MineKind::Ap,
                i,
                Team::Us,
                Vec3::new(i as f32 * 4.0, 0.0, 0.0),
                Tick(0),
                true,
            )
            .unwrap();
    }
    for i in 0..16u8 {
        mines
            .place(
                MineKind::At,
                i,
                Team::Us,
                Vec3::new(i as f32 * 4.0, 0.0, 128.0),
                Tick(0),
                true,
            )
            .unwrap();
    }
    let infantry: Vec<_> = (0..100u8)
        .map(|i| InfantryContact {
            player: i,
            pos: Vec3::new(1000.0 + f32::from(i), 0.0, 1000.0),
        })
        .collect();
    let vehicles: Vec<_> = (0..20u16)
        .map(|i| VehicleContact {
            id: i,
            pos: Vec3::new(1000.0 + f32::from(i), 0.0, 1100.0),
            mass_t: 20.0,
            trigger_side: 0,
            lightweight_destruction: false,
        })
        .collect();
    let iterations = 20_000u32;
    let start = Instant::now();
    for tick in 0..iterations {
        black_box(mines.step(Tick(tick), &infantry, &vehicles));
    }
    let ns = start.elapsed().as_nanos() as f64 / f64::from(iterations);
    println!("36 mines, 100 players + 20 vehicles: {ns:.1} ns/tick");
}
