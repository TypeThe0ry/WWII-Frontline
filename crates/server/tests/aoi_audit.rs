//! M1-024: R3 audit over a 20-client 10v10 scene with moving, turning players, plus the team
//! channel (size, period, round trip).

mod common;

use std::collections::{BTreeMap, VecDeque};

use common::sim_harness::terrain;
use common::synth::*;
use frontline_protocol::msg::team::decode_team_state;
use frontline_server::snapshot::team_channel::{
    dequant_map, facing8, quant_map, MapFrame, TeamChannel, STATE_ALIVE,
};
use frontline_server::snapshot::LinkInfo;
use frontline_sim::{GameData, Team, Vec3};

/// Independent re-statement of the R3 bound (leave limits): within 105.6 m, or within 440 m
/// and inside 110 deg of the current or three-tick-old orientation.
fn allowed(dist: f32, cos_best: f32) -> bool {
    dist <= 105.6 || (dist <= 440.0 && cos_best >= 110f32.to_radians().cos() - 1e-4)
}

#[test]
fn no_enemy_is_sent_outside_the_r3_rules() {
    let Some(map) = terrain() else { return };
    let mut s = Synth::new(new_world(Some(&map)));
    spawn_squads(&mut s, &map, 20, 20, LinkInfo::DATAGRAM);
    let mut hist: BTreeMap<u8, VecDeque<(u32, f32, f32)>> = BTreeMap::new();
    let mut checked = 0u64;
    let mut beyond_close = 0u64;
    for _ in 0..900 {
        s.step(common::sim_harness::gen_cmd);
        let t = s.tick;
        for c in s.clients.iter() {
            let Some(me) = s.world.player_state(c.player) else {
                continue;
            };
            let h = hist.entry(c.player).or_default();
            h.push_back((t, me.yaw, me.pitch));
            while h.len() > 4 {
                h.pop_front();
            }
            let Some(l) = c.log.last().filter(|l| l.tick == t) else {
                continue;
            };
            for id in &l.ids {
                let Some(o) = s.world.player_state(*id) else {
                    continue;
                };
                let Some(team) = s
                    .world
                    .view()
                    .players()
                    .iter()
                    .find(|p| p.player == *id)
                    .map(|p| p.team)
                else {
                    continue;
                };
                if team == c.team {
                    continue;
                }
                let d = o.pos - me.pos;
                let eye = me.pos + Vec3::new(0.0, 1.6, 0.0);
                let to = o.pos + Vec3::new(0.0, 1.0, 0.0) - eye;
                let dist = d.length();
                let best = h
                    .iter()
                    .filter(|&&(tick, _, _)| tick == t || t.wrapping_sub(tick) == 3)
                    .map(|&(_, y, p)| {
                        let cp = p.cos();
                        Vec3::new(y.sin() * cp, p.sin(), y.cos() * cp).dot(to) / to.length()
                    })
                    .fold(-1.0f32, f32::max);
                checked += 1;
                if dist > 105.6 {
                    beyond_close += 1;
                }
                assert!(allowed(dist, best), "leak: {dist} m, cos {best}, tick {t}");
            }
        }
    }
    let mut ns = s.sched.stats.pass_ns.clone();
    ns.sort_unstable();
    eprintln!(
        "enemy records checked {checked} ({beyond_close} beyond 105.6 m); scheduler pass for 20 clients: p50 {} us, p99 {} us",
        ns[ns.len() / 2] / 1000,
        ns[ns.len() * 99 / 100] / 1000
    );
    assert!(beyond_close > 0, "scene never exercised the cone limit");
    assert_eq!(s.sched.stats.enemy_leaks, 0, "scheduler audit counter");
}

#[test]
fn team_channel_is_small_periodic_and_round_trips() {
    let map = MapFrame {
        origin_x: 0.0,
        origin_z: 0.0,
        extent_m: 2048.0,
    };
    let mut s = Synth::new(new_world(None));
    let data = GameData::embedded();
    for i in 0..49u8 {
        s.world.roster_add(i, Team::Us).unwrap();
        s.world.roster_add(100 + i, Team::Ger).unwrap();
        s.add(
            i,
            Team::Us,
            Vec3::new(100.0 + f32::from(i) * 30.0, 5.0, 300.0),
            1.0,
            None,
        );
    }
    let clients: Vec<_> = (0..49u8).map(|p| (p, Team::Us)).collect();
    let mut ch = TeamChannel::new(1.0, 30.0, map);
    let mut sent_ticks = Vec::new();
    let mut last = None;
    for _ in 0..95 {
        s.step(|_, t| {
            let mut c = frontline_protocol::msg::input::InputCmd {
                seq: t as u16,
                tick: t,
                move_x: 0,
                move_z: 0,
                yaw: 0,
                pitch: 0,
                buttons: 0,
                weapon_slot: 0,
                aux: 0,
                view_tick: 2,
            };
            c.yaw = 0;
            c
        });
        let out = ch.tick(&s.world, data, &clients, s.tick);
        if !out.is_empty() {
            sent_ticks.push(s.tick);
            assert_eq!(out.len(), 49);
            last = Some(out[0].1.clone());
        }
    }
    let gaps: Vec<u32> = sent_ticks.windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(sent_ticks, [30, 60, 90]);
    assert!(gaps.iter().all(|&g| g == 30), "{gaps:?}");
    let bytes = last.unwrap();
    assert!(bytes.len() <= 200, "{} B", bytes.len());
    eprintln!(
        "TeamState: {} B for 49 members, ticks {sent_ticks:?}",
        bytes.len()
    );
    let ts = decode_team_state(&bytes).unwrap();
    assert_eq!(ts.members.len(), 49);
    let m = ts.members[3];
    assert_eq!(m.state, STATE_ALIVE);
    assert_eq!(m.facing, 0); // inputs face +z
    assert_eq!(facing8(1.0), 1);
    assert_eq!(facing8(core::f32::consts::PI), 4);
    assert_eq!(facing8(6.2), 0);
    let x = dequant_map(m.x, 0.0, 2048.0);
    assert!((x - (100.0 + 3.0 * 30.0)).abs() <= 1.0, "{x}");
    assert_eq!(m.x, quant_map(190.0, 0.0, 2048.0));
}
