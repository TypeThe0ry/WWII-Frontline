use super::{build_loop, InputEnvelope, InputSender, SimLoop, SimThreadConfig};
use crate::clock::fake::FakeClock;
use crate::metrics::Metrics;
use crate::session::REPEAT_MAX_GAP;
use crate::shutdown::StopFlag;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_sim::squad::{SquadKind, SquadOp};
use frontline_sim::{EntityId, GameMode, MapId, PlayerId, SpawnReq, Team, Vec3};

const EXIT_TICKS: u32 = 24; // 0.8 s at the server's 30 Hz.

struct Fixture {
    lp: SimLoop<FakeClock>,
    tx: InputSender,
}

impl Fixture {
    fn new() -> Self {
        let cfg = SimThreadConfig::bare(9, MapId(1), GameMode::Sandbox);
        let (lp, tx, _) = build_loop(FakeClock::new(), &cfg, Metrics::new(), StopFlag::new());
        let mut f = Self { lp, tx };
        f.add_player(0, true);
        f
    }

    fn add_player(&mut self, player: PlayerId, crewman: bool) {
        let lp = &mut self.lp;
        lp.bridge
            .spawn(
                &mut lp.world,
                SpawnReq {
                    player,
                    team: Team::Us,
                    pos: Vec3::new(32.0, 0.0, 32.0),
                    yaw: 0.0,
                },
            )
            .unwrap();
        lp.world.roster_add(player, Team::Us).unwrap();
        lp.world.roster_add(100 + player, Team::Ger).unwrap();
        if crewman {
            lp.world
                .squad_op(
                    player,
                    SquadOp::Create {
                        kind: SquadKind::ArmorCrew,
                        suffix: String::new(),
                    },
                )
                .unwrap();
            lp.world.set_role(player, Some("crewman")).unwrap();
        }
    }

    fn tank(&mut self, team: Team, offset: f32) -> EntityId {
        self.lp
            .world
            .spawn_vehicle("us_m4a1_75", team, Vec3::new(32.0, 0.0, 32.0 + offset), 0.0)
            .unwrap()
    }

    fn tick(&mut self, levels: &[(PlayerId, bool)]) {
        let tick = self.lp.world.tick().0 + 1;
        for &(player, pressed) in levels {
            assert!(self.tx.try_send(InputEnvelope {
                player,
                cmd: InputCmd {
                    seq: tick as u16,
                    tick,
                    move_x: 0,
                    move_z: 0,
                    yaw: 0,
                    pitch: 0,
                    buttons: if pressed { buttons::VEHICLE_SEAT } else { 0 },
                    weapon_slot: 0,
                    aux: 0,
                    view_tick: 0,
                },
            }));
        }
        self.lp.tick();
    }

    fn seat(&self, player: PlayerId) -> Option<(EntityId, u8)> {
        self.lp.world.seat_of(player)
    }

    fn board_then_release(&mut self) -> EntityId {
        let vehicle = self.tank(Team::Us, 4.0);
        self.tick(&[(0, true)]);
        self.tick(&[(0, false)]);
        assert_eq!(self.seat(0), Some((vehicle, 0)));
        vehicle
    }
}

#[test]
fn rising_edge_boards_the_nearest_reachable_friendly_vehicle() {
    let mut f = Fixture::new();
    f.tank(Team::Us, 20.0);
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None, "out of boarding range");

    f.tank(Team::Us, 6.0);
    let nearest = f.tank(Team::Us, 4.0);
    f.tank(Team::Ger, 2.0);
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None, "a held key is not a fresh edge");
    f.tick(&[(0, false)]);
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), Some((nearest, 0)));
}

#[test]
fn a_crewman_cannot_board_an_enemy_tank() {
    let mut f = Fixture::new();
    f.tank(Team::Ger, 4.0);
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None);
}

#[test]
fn a_non_crewman_cannot_board_a_friendly_tank() {
    let mut f = Fixture::new();
    f.add_player(1, false);
    f.tank(Team::Us, 4.0);
    f.tick(&[(1, true)]);
    assert_eq!(f.seat(1), None);
}

#[test]
fn occupied_seats_fall_back_to_gunner_then_commander() {
    let mut f = Fixture::new();
    f.add_player(1, true);
    f.add_player(2, true);
    f.add_player(3, true);
    let vehicle = f.tank(Team::Us, 4.0);
    f.tick(&[(0, true), (1, true), (2, true), (3, true)]);
    assert_eq!(f.seat(0), Some((vehicle, 0)));
    assert_eq!(f.seat(1), Some((vehicle, 1)));
    assert_eq!(f.seat(2), Some((vehicle, 2)));
    assert_eq!(f.seat(3), None, "all three seats are occupied");
}

#[test]
fn exit_completes_at_800_ms_and_holding_does_not_reboard() {
    let mut f = Fixture::new();
    let vehicle = f.board_then_release();
    for _ in 0..EXIT_TICKS - 1 {
        f.tick(&[(0, true)]);
        assert_eq!(f.seat(0), Some((vehicle, 0)));
    }
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None, "24 held ticks finish the exit");
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None, "continued hold is not a boarding edge");
    f.tick(&[(0, false)]);
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), Some((vehicle, 0)), "fresh press boards again");
}

#[test]
fn releasing_on_the_deadline_tick_cancels_exit_and_resets_the_hold() {
    let mut f = Fixture::new();
    let vehicle = f.board_then_release();
    for _ in 0..EXIT_TICKS - 1 {
        f.tick(&[(0, true)]);
    }
    f.tick(&[(0, false)]);
    assert_eq!(f.seat(0), Some((vehicle, 0)));
    for _ in 0..EXIT_TICKS - 1 {
        f.tick(&[(0, true)]);
        assert_eq!(f.seat(0), Some((vehicle, 0)));
    }
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None);
}

#[test]
fn short_input_gaps_preserve_the_hold_but_stale_input_cancels_it() {
    let mut f = Fixture::new();
    let vehicle = f.board_then_release();
    for _ in 0..EXIT_TICKS - REPEAT_MAX_GAP - 1 {
        f.tick(&[(0, true)]);
    }
    for _ in 0..REPEAT_MAX_GAP {
        f.tick(&[]);
        assert_eq!(f.seat(0), Some((vehicle, 0)));
    }
    f.tick(&[]); // The gap becomes stale on the deadline tick.
    assert_eq!(f.seat(0), Some((vehicle, 0)));

    f.tick(&[(0, true)]);
    f.tick(&[]); // A missing tick inside the repeat window still counts toward the hold.
    for _ in 0..EXIT_TICKS - 3 {
        f.tick(&[(0, true)]);
        assert_eq!(f.seat(0), Some((vehicle, 0)));
    }
    f.tick(&[(0, true)]);
    assert_eq!(f.seat(0), None);
}
