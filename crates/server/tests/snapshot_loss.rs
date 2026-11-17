//! M0-019 acceptance 4 and 5: baselines, keyframes and event redundancy under packet loss, with
//! the protocol crate's `SnapshotDecoder` as the in-process client.

mod common;

use common::sim_harness::{gen_cmd, terrain};
use common::synth::*;
use frontline_protocol::msg::input::{buttons, InputCmd};
use frontline_server::snapshot::LinkInfo;
use frontline_sim::{Team, Vec3};

fn world_with_players(
    map: &std::sync::Arc<frontline_server::world_loader::LoadedMap>,
    loss: f64,
) -> Synth {
    let mut s = Synth::new(new_world(Some(map)));
    s.loss = loss;
    spawn_squads(&mut s, map, 12, 4, LinkInfo::DATAGRAM);
    s
}

#[test]
fn thirty_percent_loss_decodes_continuously_and_recovers_within_a_second() {
    let Some(map) = terrain() else { return };
    let mut s = world_with_players(&map, 0.30);
    for _ in 0..(30 * 60) {
        s.step(gen_cmd);
    }
    for c in &s.clients {
        let total = c.packets_ok + c.packets_lost + c.missing_baseline;
        eprintln!(
            "client {}: {total} packets, lost {} ({:.0} %), MissingBaseline {} , decoded {}, worst recovery {} ticks ({:.2} s)",
            c.player,
            c.packets_lost,
            100.0 * c.packets_lost as f64 / total as f64,
            c.missing_baseline,
            c.packets_ok,
            c.max_recovery_ticks,
            c.max_recovery_ticks as f64 / 30.0
        );
        assert!(c.packets_ok > 500, "client {} starved of packets", c.player);
        assert!(
            c.max_recovery_ticks <= 30,
            "recovery after MissingBaseline took {} ticks",
            c.max_recovery_ticks
        );
    }
    // the server never produced an undecodable packet other than MissingBaseline (decode errors
    // panic in the harness)
}

#[test]
fn ack_lag_beyond_32_ticks_yields_a_full_keyframe_packet() {
    let Some(map) = terrain() else { return };
    let mut s = world_with_players(&map, 0.0);
    s.sched.near_every_packet = true; // guarantees delta records in the scene
    for _ in 0..120 {
        s.step(gen_cmd);
    }
    let before = s.client(0).log.len();
    assert!(
        s.client(0).log[before - 5..].iter().any(|l| l.has_baseline),
        "steady state uses deltas"
    );
    // 45 ticks of downlink blackout: no acks reach the server
    s.client_mut(0).blackout = true;
    for _ in 0..45 {
        s.step(gen_cmd);
    }
    s.client_mut(0).blackout = false;
    let n = s.client(0).log.len();
    // first packet that arrives after the outage (the encoder judges baseline age by tick)
    let mut got = None;
    for _ in 0..6 {
        s.step(gen_cmd);
        if s.client(0).log.len() > n {
            got = Some(s.client(0).log[n].clone());
            break;
        }
    }
    let l = got.expect("a packet after the outage");
    eprintln!(
        "first packet after 45-tick outage: {} entities, {} keyframes, has_baseline {}",
        l.entities, l.keyframes, l.has_baseline
    );
    assert!(l.entities > 0);
    assert_eq!(l.keyframes, l.entities, "all records are keyframes");
    assert!(!l.has_baseline);
    // and the stream goes back to deltas once acks flow again
    for _ in 0..30 {
        s.step(gen_cmd);
    }
    assert!(s.client(0).log.iter().rev().take(6).any(|l| l.has_baseline));
}

fn fire_pulse(p: u8, t: u32) -> InputCmd {
    let mut c = gen_cmd(p, t);
    // shooters (ids >= 4) stand still and fire a shot every 8 ticks, reloading in between
    if p >= 4 {
        c.move_x = 0;
        c.move_z = 0;
        c.buttons = 0;
        if t.is_multiple_of(8) {
            c.buttons |= buttons::FIRE;
        }
        if t % 90 == 60 {
            c.buttons |= buttons::RELOAD;
        }
    }
    c
}

fn event_run(loss: f64) -> Option<(u64, usize, u32, f64)> {
    let map = terrain()?;
    let mut s = Synth::new(new_world(Some(&map)));
    s.loss = loss;
    // everybody on one team (friends: Shot events need no R3 view), 4 clients + 8 shooters
    for i in 0..12u8 {
        let (pos, yaw) = map.join_spawn(i, Team::Us);
        s.add(i, Team::Us, pos, yaw, (i < 4).then_some(LinkInfo::DATAGRAM));
    }
    let mut shots = 0u64;
    for _ in 0..(30 * 120) {
        let ev = s.step(fire_pulse);
        shots += ev
            .iter()
            .filter(|e| matches!(e, frontline_sim::SimEvent::Shot { .. }))
            .count() as u64;
    }
    // generous drain so that the last events had their 3 packets
    for _ in 0..10 {
        s.step(|p, t| {
            let mut c = fire_pulse(p, t);
            c.buttons = 0;
            c
        });
    }
    let mut queued = 0u64;
    let mut unique = 0usize;
    let mut max_arrivals = 0u32;
    let mut dups = 0u64;
    for c in &s.clients {
        let b = s.sched.client(c.player).unwrap();
        queued += b.events.queued_total;
        unique += c.seen_events.len();
        max_arrivals = max_arrivals.max(c.event_arrivals.values().copied().max().unwrap_or(0));
        dups += c.duplicate_events;
    }
    eprintln!(
        "loss {:.0} %: {shots} Shot events simulated, {queued} queued for the 4 clients, {unique} delivered after de-duplication, {dups} duplicate arrivals, an event arrived at most {max_arrivals} times",
        loss * 100.0
    );
    Some((queued, unique, max_arrivals, dups as f64))
}

#[test]
fn events_are_redundant_deduplicated_and_survive_loss() {
    let Some((queued, unique, max_arr, _)) = event_run(0.0) else {
        return;
    };
    assert!(queued > 200, "scene produced only {queued} events");
    assert_eq!(unique as u64, queued, "lossless: everything delivered once");
    assert!(
        max_arr <= 3,
        "an event was sent more than 3 times: {max_arr}"
    );
    let (queued, unique, max_arr, _) = event_run(0.30).unwrap();
    let rate = unique as f64 / queued as f64;
    eprintln!(
        "delivery under 30 % loss: {:.2} % ({unique}/{queued})",
        rate * 100.0
    );
    assert!(max_arr <= 3);
    assert!(rate >= 0.97, "delivery {:.2} % < 97 %", rate * 100.0);
}

#[test]
fn hit_confirm_goes_to_the_shooter_and_damage_dir_to_the_victim_only() {
    let Some(map) = terrain() else { return };
    // shooter and victim 25 m apart on the real terrain, a bystander further away
    let at = |x: f32, z: f32| Vec3::new(x, map.collision.height_at(x, z).unwrap_or(0.0) + 0.02, z);
    // aim the shooter's eye (1.65 m) at the victim's chest (1.235 m) 15 m away
    let (a, b) = (at(110.0, 100.0), at(110.0, 115.0));
    let pitch =
        frontline_protocol::quant::rad_to_pitch_i16(((b.y + 1.235) - (a.y + 1.65)).atan2(15.0))
            .code;
    let mut s = Synth::new(new_world(Some(&map)));
    s.add(1, Team::Us, a, 0.0, Some(LinkInfo::DATAGRAM));
    s.add(
        2,
        Team::Ger,
        b,
        std::f32::consts::PI,
        Some(LinkInfo::DATAGRAM),
    );
    s.add(
        3,
        Team::Ger,
        at(160.0, 100.0),
        0.0,
        Some(LinkInfo::DATAGRAM),
    );
    let mut hits = 0;
    for t in 0..300 {
        let ev = s.step(|p, tick| {
            let mut c = common::sim_harness::gen_cmd(p, tick);
            c.move_x = 0;
            c.move_z = 0;
            c.yaw = if p == 2 { 32768 } else { 0 };
            c.pitch = if p == 1 { pitch } else { 0 };
            c.buttons = if p == 1 && tick % 6 == 0 && t < 250 {
                buttons::FIRE
            } else {
                0
            };
            if p == 1 && tick % 90 == 60 {
                c.buttons |= buttons::RELOAD;
            }
            c
        });
        hits += ev
            .iter()
            .filter(|e| matches!(e, frontline_sim::SimEvent::Hit { .. }))
            .count();
    }
    assert!(hits > 0, "scene must produce hits");
    use frontline_protocol::snapshot::Event;
    let kinds = |p: u8| -> (usize, usize) {
        let c = s.client(p);
        (
            c.events
                .iter()
                .filter(|(_, e)| matches!(e.event, Event::HitConfirm { .. }))
                .count(),
            c.events
                .iter()
                .filter(|(_, e)| matches!(e.event, Event::DamageDir { .. }))
                .count(),
        )
    };
    let (hc1, dd1) = kinds(1);
    let (hc2, dd2) = kinds(2);
    let (hc3, dd3) = kinds(3);
    eprintln!("hits {hits}: shooter HitConfirm/DamageDir = {hc1}/{dd1}, victim = {hc2}/{dd2}, bystander = {hc3}/{dd3}");
    assert!(hc1 >= 1 && dd1 == 0, "shooter");
    assert!(dd2 >= 1 && hc2 == 0, "victim");
    assert_eq!((hc3, dd3), (0, 0), "bystander");
}
