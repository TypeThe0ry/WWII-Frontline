//! Synthetic player input at 30 Hz (M0-027 behaviour 3): waypoint walking, point-blank bursts every
//! 2-6 s and the odd jump / crouch. A pure function of `(seed, client index, waypoints)` and the
//! position fed back each tick, so two runs with the same `--seed` generate the same sequence when
//! fed the same positions (unit-tested, and `--hash-only` prints it for a kinematic stand-in).

use frontline_protocol::msg::input::{buttons, InputCmd};

pub const TICK_HZ: f64 = 30.0;
/// Arrival radius of a waypoint, metres.
const ARRIVE_M: f32 = 3.0;
/// Turn rate limit, radians per tick (180 deg/s).
const TURN_PER_TICK: f32 = std::f32::consts::PI / TICK_HZ as f32;
/// A client that moved less than this in `STUCK_TICKS` ticks picks a new waypoint.
const STUCK_M: f32 = 1.0;
const STUCK_TICKS: u32 = 90;

/// Deterministic xorshift64* (the generator must not depend on `rand`).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 scramble so adjacent seeds give unrelated streams
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 0x1234_5678_9ABC_DEF1 } else { z })
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi - lo + 1)) as u32
    }
}

/// What the generator knows about its player this tick.
#[derive(Debug, Clone, Copy)]
pub struct Pose {
    /// Predicted foot position (east, north) in metres; `None` before the first placement.
    pub xz: Option<(f32, f32)>,
    pub alive: bool,
}

#[derive(Debug, Clone)]
pub struct SynthInput {
    rng: Rng,
    waypoints: Vec<[f32; 2]>,
    wp: usize,
    yaw: f32,
    pitch: f32,
    until_burst: u32,
    shots_left: u32,
    shot_phase: u32,
    until_jump: u32,
    crouch_left: u32,
    until_crouch: u32,
    strafe_left: u32,
    strafe_dir: i8,
    evade_left: u32,
    stuck_ref: Option<(f32, f32)>,
    stuck_ticks: u32,
    hash: u64,
    /// Commands that fed the hash (alive and placed).
    pub active_cmds: u64,
    /// Bursts started.
    pub bursts: u64,
}

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

impl SynthInput {
    /// `client` distinguishes the clients of one process; the waypoint list may be empty (a
    /// fallback box is used then).
    pub fn new(seed: u64, client: u32, waypoints: &[[f32; 2]]) -> Self {
        let mut rng = Rng::new(seed ^ (u64::from(client) + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93));
        let wps: Vec<[f32; 2]> = if waypoints.is_empty() {
            vec![[60.0, 60.0], [196.0, 60.0], [196.0, 196.0], [60.0, 196.0]]
        } else {
            waypoints.to_vec()
        };
        let wp = rng.next_u64() as usize % wps.len();
        let yaw = rng.f32() * std::f32::consts::TAU;
        SynthInput {
            until_burst: rng.range(60, 180),
            until_jump: rng.range(450, 1200),
            until_crouch: rng.range(600, 1800),
            rng,
            waypoints: wps,
            wp,
            yaw,
            pitch: 0.0,
            shots_left: 0,
            shot_phase: 0,
            crouch_left: 0,
            strafe_left: 0,
            strafe_dir: 1,
            evade_left: 0,
            stuck_ref: None,
            stuck_ticks: 0,
            hash: FNV_OFFSET,
            active_cmds: 0,
            bursts: 0,
        }
    }

    /// Hash of every active command so far (move, yaw, pitch, buttons; not seq / tick).
    pub fn input_hash(&self) -> u64 {
        self.hash
    }

    fn pick_waypoint(&mut self) {
        let n = self.waypoints.len();
        if n > 1 {
            // a different one than the current
            let step = 1 + (self.rng.next_u64() as usize % (n - 1));
            self.wp = (self.wp + step) % n;
        }
    }

    /// Next command for `seq` / `tick`.
    pub fn next(&mut self, seq: u16, tick: u32, pose: Pose) -> InputCmd {
        let mut cmd = InputCmd {
            seq,
            tick,
            move_x: 0,
            move_z: 0,
            yaw: yaw_code(self.yaw),
            pitch: pitch_code(self.pitch),
            buttons: 0,
            weapon_slot: 0,
            aux: 0,
            view_tick: 0,
        };
        let Some((x, z)) = pose.xz.filter(|_| pose.alive) else {
            // not placed yet, or dead: stand still, no fire (the server ignores dead players)
            self.stuck_ref = None;
            self.stuck_ticks = 0;
            self.shots_left = 0;
            return cmd;
        };

        // --- steering ---------------------------------------------------------------------
        let [wx, wz] = self.waypoints[self.wp];
        let (dx, dz) = (wx - x, wz - z);
        if dx * dx + dz * dz < ARRIVE_M * ARRIVE_M {
            self.pick_waypoint();
        }
        let [wx, wz] = self.waypoints[self.wp];
        let mut want = (wx - x).atan2(wz - z);
        if self.evade_left > 0 {
            self.evade_left -= 1;
            want = self.yaw + std::f32::consts::FRAC_PI_2;
        }
        let mut d = (want - self.yaw).rem_euclid(std::f32::consts::TAU);
        if d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        self.yaw = (self.yaw + d.clamp(-TURN_PER_TICK, TURN_PER_TICK))
            .rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + (self.rng.f32() - 0.5) * 0.01).clamp(-0.12, 0.12);
        cmd.yaw = yaw_code(self.yaw);
        cmd.pitch = pitch_code(self.pitch);
        cmd.move_z = 127;
        // lateral wiggle
        if self.strafe_left > 0 {
            self.strafe_left -= 1;
            cmd.move_x = 64 * self.strafe_dir;
        } else if self.rng.range(0, 299) == 0 {
            self.strafe_left = self.rng.range(15, 45);
            self.strafe_dir = if self.rng.range(0, 1) == 0 { 1 } else { -1 };
        }

        // --- stuck detection --------------------------------------------------------------
        match self.stuck_ref {
            None => {
                self.stuck_ref = Some((x, z));
                self.stuck_ticks = 0;
            }
            Some((sx, sz)) => {
                self.stuck_ticks += 1;
                if self.stuck_ticks >= STUCK_TICKS {
                    if (x - sx).hypot(z - sz) < STUCK_M {
                        self.pick_waypoint();
                        self.evade_left = 30;
                    }
                    self.stuck_ref = Some((x, z));
                    self.stuck_ticks = 0;
                }
            }
        }

        // --- fire: a burst of 1-3 shots, each FIRE held 3 ticks then released 3 ticks ---------
        if self.shots_left == 0 {
            if self.until_burst == 0 {
                self.shots_left = self.rng.range(1, 3);
                self.shot_phase = 0;
                self.until_burst = self.rng.range(60, 180); // 2-6 s
                self.bursts += 1;
            } else {
                self.until_burst -= 1;
            }
        }
        if self.shots_left > 0 {
            if self.shot_phase < 3 {
                cmd.buttons |= buttons::FIRE;
            }
            self.shot_phase += 1;
            if self.shot_phase >= 6 {
                self.shot_phase = 0;
                self.shots_left -= 1;
            }
        }

        // --- jump / crouch ----------------------------------------------------------------
        if self.until_jump == 0 {
            cmd.buttons |= buttons::JUMP;
            self.until_jump = self.rng.range(450, 1200); // 15-40 s
        } else {
            self.until_jump -= 1;
        }
        if self.crouch_left > 0 {
            self.crouch_left -= 1;
            cmd.buttons |= buttons::CROUCH;
        } else if self.until_crouch == 0 {
            self.crouch_left = 60;
            self.until_crouch = self.rng.range(600, 1800); // 20-60 s
        } else {
            self.until_crouch -= 1;
        }

        let mut h = self.hash;
        h = fnv(h, &[cmd.move_x as u8, cmd.move_z as u8]);
        h = fnv(h, &cmd.yaw.to_le_bytes());
        h = fnv(h, &cmd.pitch.to_le_bytes());
        h = fnv(h, &cmd.buttons.to_le_bytes());
        self.hash = h;
        self.active_cmds += 1;
        cmd
    }
}

/// Radians -> 16-bit angle over `[0, 2 pi)`.
pub fn yaw_code(rad: f32) -> u16 {
    ((rad.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU) * 65536.0) as u32 as u16
}

/// Radians in `[-pi/2, pi/2]` -> `i16`.
pub fn pitch_code(rad: f32) -> i16 {
    (rad / std::f32::consts::FRAC_PI_2 * 32767.0).clamp(-32767.0, 32767.0) as i16
}

/// Runs the generator for `ticks` ticks on a kinematic stand-in (5 m/s along the commanded yaw,
/// no terrain): a deterministic input-sequence fingerprint for `--hash-only` and tests.
pub fn kinematic_hash(seed: u64, client: u32, waypoints: &[[f32; 2]], ticks: u32) -> u64 {
    let mut g = SynthInput::new(seed, client, waypoints);
    let (mut x, mut z) = (waypoints.first().map_or(128.0, |p| p[0]), 100.0f32);
    for t in 0..ticks {
        let c = g.next(
            t as u16,
            t,
            Pose {
                xz: Some((x, z)),
                alive: true,
            },
        );
        let yaw = f32::from(c.yaw) / 65536.0 * std::f32::consts::TAU;
        let v = f32::from(c.move_z) / 127.0 * 5.0 / TICK_HZ as f32;
        x += yaw.sin() * v;
        z += yaw.cos() * v;
    }
    g.input_hash()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WPS: [[f32; 2]; 3] = [[40.0, 40.0], [216.0, 40.0], [128.0, 200.0]];

    #[test]
    fn same_seed_same_hash_different_seed_or_client_differs() {
        let a = kinematic_hash(1, 0, &WPS, 9000);
        assert_eq!(a, kinematic_hash(1, 0, &WPS, 9000));
        assert_ne!(a, kinematic_hash(2, 0, &WPS, 9000));
        assert_ne!(a, kinematic_hash(1, 1, &WPS, 9000));
        assert_ne!(a, kinematic_hash(1, 0, &WPS, 9001));
    }

    #[test]
    fn walks_towards_a_waypoint_and_reaches_it() {
        let wps = [[100.0f32, 100.0]];
        let mut g = SynthInput::new(7, 0, &wps);
        let (mut x, mut z) = (60.0f32, 60.0f32);
        let mut best = f32::MAX;
        for t in 0..1800u32 {
            let c = g.next(
                t as u16,
                t,
                Pose {
                    xz: Some((x, z)),
                    alive: true,
                },
            );
            let yaw = f32::from(c.yaw) / 65536.0 * std::f32::consts::TAU;
            x += yaw.sin() * 5.0 / 30.0 * f32::from(c.move_z) / 127.0;
            z += yaw.cos() * 5.0 / 30.0 * f32::from(c.move_z) / 127.0;
            best = best.min((x - 100.0).hypot(z - 100.0));
        }
        assert!(best < ARRIVE_M + 1.5, "closest approach {best} m");
    }

    #[test]
    fn dead_or_unplaced_is_neutral_and_not_hashed() {
        let mut g = SynthInput::new(1, 0, &WPS);
        let c = g.next(
            1,
            1,
            Pose {
                xz: None,
                alive: true,
            },
        );
        assert_eq!((c.move_x, c.move_z, c.buttons), (0, 0, 0));
        let c = g.next(
            2,
            2,
            Pose {
                xz: Some((1.0, 1.0)),
                alive: false,
            },
        );
        assert_eq!((c.move_x, c.move_z, c.buttons), (0, 0, 0));
        assert_eq!(g.active_cmds, 0);
        assert_eq!(g.input_hash(), FNV_OFFSET);
    }

    #[test]
    fn fire_bursts_every_2_to_6_seconds_with_rising_edges() {
        let mut g = SynthInput::new(3, 0, &WPS);
        let mut last_fire = false;
        let mut edges = Vec::new();
        for t in 0..30 * 120u32 {
            let c = g.next(
                t as u16,
                t,
                Pose {
                    xz: Some((100.0, 100.0)),
                    alive: true,
                },
            );
            let f = c.buttons & buttons::FIRE != 0;
            if f && !last_fire {
                edges.push(t);
            }
            last_fire = f;
        }
        assert!(edges.len() >= 120 / 6, "{} shots in 120 s", edges.len());
        // bursts (started `bursts` times) are 2-6 s apart minus the burst's own length (<= 18 ticks)
        assert!(g.bursts >= 20 && g.bursts <= 61, "{} bursts", g.bursts);
    }

    #[test]
    fn commands_are_valid_for_the_codec() {
        let mut g = SynthInput::new(9, 4, &WPS);
        for t in 0..3000u32 {
            let c = g.next(
                t as u16,
                t,
                Pose {
                    xz: Some((50.0 + (t % 50) as f32, 50.0)),
                    alive: true,
                },
            );
            assert!(c.move_x > i8::MIN && c.move_z > i8::MIN);
            assert!(c.pitch.unsigned_abs() <= 32767);
            assert_eq!(c.buttons & !0x3FFF, 0);
        }
    }

    #[test]
    fn angle_codes() {
        assert_eq!(yaw_code(0.0), 0);
        assert_eq!(yaw_code(std::f32::consts::PI), 32768);
        assert_eq!(yaw_code(-std::f32::consts::FRAC_PI_2), 49152);
        assert_eq!(pitch_code(0.0), 0);
        assert_eq!(pitch_code(std::f32::consts::FRAC_PI_2), 32767);
    }
}
