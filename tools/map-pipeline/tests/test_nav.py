import json
import shutil
import subprocess
from pathlib import Path

import numpy as np
import pytest

from nav import inputs, offmesh, run as nav_run

ROOT = Path(__file__).resolve().parents[3]
NODE_DIR = ROOT / "tools/nav-bake"


def test_params_match_arch():
    p = nav_run.PARAMS
    w = p["walk"]
    assert (w["cell_size"], w["cell_height"], w["agent_radius"], w["agent_height"], w["max_climb"], w["max_slope"]) == (0.3, 0.2, 0.4, 1.8, 0.5, 45)
    assert p["tile_size_m"] == 128 and p["vehicle"]["agent_radius"] == 2.0 and p["vehicle"]["max_slope"] == 25
    on_disk = json.loads((ROOT / "data/maps/carentan/m1/navmesh_params.json").read_text())
    assert on_disk["params_hash"] == nav_run.params_hash()


def test_terrain_holes_and_winding():
    h = np.zeros((20, 20))
    hole = np.zeros((20, 20), dtype=bool)
    hole[5, 5] = True
    pos, tri = inputs.terrain(h, hole, 0, 0, 10, 10)
    assert len(tri) == 2 * (10 * 10 - 1) and len(pos) == 11 * 11
    a, b, c = (pos[i] for i in tri[0])
    n = np.cross(b - a, c - a)
    assert n[1] > 0  # y-up winding


def test_door_cut():
    pieces = inputs._cut((0, 0), (10, 0), (5, 0), 1.0)
    assert pieces == [((0, 0), (4.0, 0.0)), ((6.0, 0.0), (10, 0))]


def test_offmesh_validation():
    ok = {"id": "a", "kind": "stairs", "start": [0, 0, 0], "end": [1, 3, 0], "radius": 0.4, "bidirectional": True, "cost": 2.0}
    assert offmesh.validate([ok], lambda *p: 0.1) == []
    bad = dict(ok, id="b", kind="teleport", cost=0.5)
    errs = offmesh.validate([ok, bad, dict(ok)], lambda *p: 0.9)
    joined = " ".join(errs)
    assert "bad kind" in joined and "cost" in joined and "duplicate id" in joined and "0.90 m from the navmesh" in joined


@pytest.mark.skipif(not (NODE_DIR / "node_modules").exists() or not shutil.which("node"), reason="needs tools/nav-bake deps")
def test_fixture_bake_connects_across_a_wall(tmp_path):
    n = 80
    h = np.zeros((n, n))
    hole = np.zeros((n, n), dtype=bool)
    hole[:, 38:41] = True            # a wall of holes with a gap
    hole[30:40, 38:41] = False
    pos, tri = inputs.terrain(h, hole, 0, 0, 79, 79)
    inputs.write(tmp_path / "t.bin", pos, tri)
    (tmp_path / "p.json").write_text(json.dumps({"tile_size_m": 32, "origin": [0, 0, 0], "walk": nav_run.PARAMS["walk"], "vehicle": nav_run.PARAMS["vehicle"]}))
    pts = {"paths": {"across": [[10, 0.2, 10], [70, 0.2, 10]], "gap": [[10, 0.2, 35], [70, 0.2, 35]]}}
    (tmp_path / "pts.json").write_text(json.dumps(pts))
    r = subprocess.run(["node", "src/bake.mjs", "--in", str(tmp_path / "t.bin"), "--params", str(tmp_path / "p.json"), "--mode", "walk",
                        "--out", str(tmp_path / "n.bin"), "--report", str(tmp_path / "r.json"), "--points", str(tmp_path / "pts.json")],
                       cwd=NODE_DIR, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    rep = json.loads((tmp_path / "r.json").read_text())
    assert rep["paths"]["gap"]["ok"] and rep["paths"]["across"]["ok"]
    assert rep["paths"]["across"]["length_m"] > 62  # forced to detour through the gap
    assert (tmp_path / "n.bin").read_bytes()[:4] == b"FNAV"


def test_real_report():
    r = json.loads((ROOT / "reports/m1-navmesh.json").read_text())
    assert r["connectivity_all_ok"]
    for m in ("walk", "vehicle"):
        assert all(v["ok"] and v["end_gap_m"] < 3.0 for v in r[m]["paths"].values()), "partial paths must not count as connected"
    assert r["walk"]["buildings"]["unreachable"] <= 0.05 * r["walk"]["buildings"]["n"]
    assert r["walk"]["water"]["covered"] == 0 and r["walk"]["road"]["covered"] / r["walk"]["road"]["n"] >= 0.99
    assert r["walk"]["bytes"] <= 6 * 1024 * 1024 and r["walk"]["tiles"] == 100
    assert r["walk"]["params_hash"] == r["vehicle"]["params_hash"]
    head = (ROOT / "data/maps/carentan/m1/navmesh.bin").read_bytes()[:16]
    assert head[:4] == b"FNAV" and head[8:16].hex() == nav_run.params_hash()
