"""Install the B6 fleet workload into a disposable MS source snapshot."""

import copy
import json
from pathlib import Path
import subprocess
import sys


PLAYABLE = "b2c5386022683c0ab375ffb2084ca81a612294e0"


def replace_once(path, old, new):
    source = path.read_text()
    if source.count(old) != 1:
        raise ValueError(f"{path}: expected exactly one fixture insertion point")
    path.write_text(source.replace(old, new))


def install(source):
    vehicles = source / "data/vehicles"
    sherman = json.loads((vehicles / "us_m4a1_75.json").read_text())
    for name in ("ger_panzer_iv_h", "us_jeep", "ger_kubel"):
        donor = json.loads(subprocess.check_output(
            ["git", "show", f"{PLAYABLE}:data/vehicles/{name}.json"], text=True
        ))
        # The MS physics model only accepts tracked-body parameters. Approximate the
        # missing vehicles with that solver. The overlay is for stationary C2 load only.
        fixture = copy.deepcopy(sherman)
        for field in ("id", "name_key", "type", "faction", "historical"):
            fixture[field] = donor[field]
        gameplay = fixture["gameplay"]
        playable_gameplay = donor["gameplay"]
        body = playable_gameplay["playable"]
        fixture["historical"]["dimensions_m"] = {
            "length": 2 * body["hull_half_length_m"],
            "width": 2 * body["hull_half_width_m"],
            "height": body["hull_height_m"],
        }
        for field in ("hull_health_hp", "armor_mm", "seats", "spawn_cost", "respawn_cooldown_s"):
            gameplay[field] = playable_gameplay[field]
        armor = playable_gameplay.get("armor_zones", {})
        for zone in gameplay["armor_zones"]:
            if zone["id"] in armor:
                zone.update(armor[zone["id"]])
            elif name != "ger_panzer_iv_h":
                zone["thickness_mm"] = 1
                zone["slope_deg"] = 0
        gameplay["max_speed_mps"] = body["max_speed_mps"]
        gameplay["hull_turn_deg_s"] = body["turn_deg_s"]
        gameplay["max_slope_deg"] = body["max_slope_deg"]
        mass_ratio = fixture["historical"]["mass_kg"] / sherman["historical"]["mass_kg"]
        for field in ("stiffness_n_per_m", "damping_ns_per_m"):
            gameplay["suspension"][field] *= mass_ratio
        # Valid cannon schema for the MS constructor, with no ammunition or shell data.
        gameplay["weapons"] = [{
            "kind": "cannon", "reload_s": 3, "switch_reload_s": 1,
            "ammo_rounds": {}, "shells": [],
        }]
        (vehicles / f"{name}.json").write_text(json.dumps(fixture, indent=2) + "\n")

    sim = source / "crates/server/src/sim_thread.rs"
    replace_once(sim, "        if cfg.dev_spawn_vehicle {", """        if m.name == "carentan/m1" && m.uses_tactical_deploy() {
            for (team, roster) in [
                (Team::Us, ["us_m4a1_75", "us_jeep"]),
                (Team::Ger, ["ger_panzer_iv_h", "ger_kubel"]),
            ] {
                let hq = m.spawns.hq[team as usize];
                let (_, yaw) = m.join_spawn(0, team);
                for (index, def_id) in roster.iter().enumerate() {
                    let lateral = if index == 0 { -8.0 } else { 8.0 };
                    let x = hq.x + lateral * yaw.cos() + 12.0 * yaw.sin();
                    let z = hq.z - lateral * yaw.sin() + 12.0 * yaw.cos();
                    let ground = m.collision.height_at(x, z).expect("B6 vehicle pad terrain");
                    let pos = frontline_sim::Vec3::new(x, ground + 0.9, z);
                    let id = world.spawn_vehicle(def_id, team, pos, yaw).expect("B6 fleet");
                    tracing::info!(event = "b6_fleet_spawn", def_id, ?team, ?id, x, y = pos.y, z);
                }
            }
        }
        if cfg.dev_spawn_vehicle {""")
    replace_once(sim, "        self.step_events = out.events;", """        if self.world.tick().0.is_multiple_of(300) {
            let views = self.world.vehicle_views();
            if !views.is_empty() {
                tracing::info!(event = "b6_fleet_sample", tick = self.world.tick().0,
                    vehicles = views.len(), healthy = views.iter().filter(|v| v.hull_hp > 0.0).count(),
                    drivers = views.iter().filter(|v| v.driver.is_some()).count());
            }
        }
        self.step_events = out.events;""")

    audit = source / "crates/headless-client/src/audit.rs"
    wire_end = '        eprintln!("{line}");\n    }\n\n    /// Checks the legacy'
    replace_once(audit, wire_end, """        eprintln!("{line}");
        if tick % 300 <= 2 {
            let slots: Vec<_> = (0..e.len())
                .filter(|&i| e.kind[i] == EntityKind::Vehicle)
                .map(|i| e.ids[i])
                .collect();
            if !slots.is_empty() {
                eprintln!("[b6-fleet-wire] {player} {tick} {slots:?}");
            }
        }
    }

    /// Checks the legacy""")

    client = source / "crates/headless-client/src/client.rs"
    replace_once(client, "    let bind = ctx", """    static LOAD_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let load_start = *LOAD_START.get_or_init(Instant::now);
    let bind = ctx""")
    replace_once(client, "    if ended_by_us {", """    eprintln!("[b6-window] {} {} {:.6} {:.6}", id, ended_by_us,
        t_start.saturating_duration_since(load_start).as_secs_f64(),
        load_start.elapsed().as_secs_f64());
    if ended_by_us {""")


if __name__ == "__main__":
    snapshot = Path(sys.argv[1]).resolve()
    if snapshot == Path.cwd().resolve() or (snapshot / ".git").exists():
        raise SystemExit("Run from the milestone worktree against a disposable git archive")
    install(snapshot)
