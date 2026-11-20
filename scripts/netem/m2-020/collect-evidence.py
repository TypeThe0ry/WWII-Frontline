#!/usr/bin/env python3
"""Emit one compact evidence JSON object per completed M2-020 fixture directory."""

import argparse
import json
from pathlib import Path

from run import metric


SCRAPE_PREFIXES = (
    "net_bytes_out_per_client_kbps_", "net_budget_scale_",
    "net_regulated_clients ", "net_far_shed_total ", "ws_snapshot_dropped_total ",
    "aoi_starved_total ", "server_players{", "server_bots ", "server_tick_ms_count ",
)
CLIENT_KEYS = (
    "player_id", "team", "hq_spawn_tick", "connected_s", "join_error",
    "disconnects", "snapshots", "team_packets", "decode_errors", "missing_baseline",
)


def collect(path):
    def load(name):
        return json.loads((path / f"{name}.json").read_text())

    def scrape(sample):
        return (path / sample["file"]).read_text()

    def endpoint(sample):
        return {"elapsed_s": sample["elapsed_s"], "prometheus": [
            line for line in scrape(sample).splitlines() if line.startswith(SCRAPE_PREFIXES)
        ]}

    timeline = [json.loads(line) for line in (path / "samples.jsonl").read_text().splitlines()]
    window_start = next(s for s in reversed(timeline) if s["phase"] == "warmup")
    measurements = [s for s in timeline if s["phase"] == "measure"]
    server, client = load("server"), load("headless")
    result = {
        "run": load("run"), "summary": load("summary"),
        "qdisc_start": load("qdisc-start"), "qdisc_end": load("qdisc-end"),
        "metric_start": endpoint(window_start), "metric_end": endpoint(measurements[-1]),
        "populations_in_window": [[round(s["elapsed_s"], 6), s["sessions"], s["bots"]] for s in measurements],
        "server_report": server,
        "clients": [{k: c.get(k) for k in CLIENT_KEYS} for c in client["per_client"]],
        "client_aggregate": client["aggregate"],
    }
    if (path / "restored.json").exists():
        restored = load("restored")
        result["netem_removed"] = restored
        result["recovery_regulated_clients"] = [
            [round(s["elapsed_s"] - restored["elapsed_s"], 6), metric(scrape(s), "net_regulated_clients")]
            for s in timeline if s["phase"] == "recovery"
        ]
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path, nargs="+")
    for run in parser.parse_args().run:
        print(json.dumps(collect(run), separators=(",", ":")))
