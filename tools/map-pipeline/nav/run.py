"""`map-pipeline nav --slice m1`: inputs -> Node bake (tools/nav-bake) -> data + report (M1-036)."""
from __future__ import annotations

import hashlib
import json
import random
import subprocess
import time
from pathlib import Path

import numpy as np

from map_pipeline import buildings_annotate as ba, zones
from . import inputs, offmesh

ARCH = {"cell_size": 0.3, "cell_height": 0.2, "agent_radius": 0.4, "agent_height": 1.8, "max_climb": 0.5, "max_slope": 45}
PARAMS = {
    "schema": "frontline.map.navmesh_params/1", "tile_size_m": 128, "origin": [380.0, 0.0, 520.0],
    "walk": dict(ARCH),
    "vehicle": {"cell_size": 0.5, "cell_height": 0.25, "agent_radius": 2.0, "agent_height": 2.6, "max_climb": 0.6, "max_slope": 25},
    "areas": {"ground": 1, "wade": 2, "door": 3, "water": 0},
    "kcc_walk_slope_limit_deg": 35,
    "note": "walk params follow ARCHITECTURE 16.2 (MAPS 6.2 differs: PI-085); wade/door areas are not marked yet (PI-085)",
}


def params_hash() -> str:
    return hashlib.sha256(json.dumps(PARAMS, sort_keys=True).encode()).hexdigest()[:16]


def _y(h, x, z):
    return float(h[int(z), int(x)]) + 0.2


def points(out: Path, h, seed=7) -> dict:
    z = json.loads((out / "m1" / "zones.json").read_text(encoding="utf-8"))
    sp = json.loads((out / "m1" / "spawns.json").read_text(encoding="utf-8"))
    P = {p["id"]: p["pos"] for line in z["lines"] for p in line["points"]}
    us, ger = sp["hq"]["US"]["pos"], sp["hq"]["GER"]["pos"]
    tgt = {"us_hq_to_L1a": (us, P["L1a"]), "us_hq_to_L2a": (us, P["L2a"]), "us_hq_to_L3a": (us, P["L3a"]), "us_hq_to_ger_hq": (us, ger)}
    paths = {k: [[a[0], _y(h, *a), a[1]], [b[0], _y(h, *b), b[1]]] for k, (a, b) in tgt.items()}
    layers = {k: json.loads((out / f"{k}.geojson").read_text(encoding="utf-8"))["features"] for k in ("buildings", "water", "roads")}
    g = zones.build_grid([], layers["water"], res=1.0)
    rng = random.Random(seed)
    R = ba.RECT
    deep = np.argwhere(g.b[int(R[1]) + 6 : int(R[3]) - 6, int(R[0]) + 6 : int(R[2]) - 6])
    # deep water = water cell whose 6 m neighbourhood is entirely water-blocked
    cand = [(c + int(R[0]) + 6 + 0.5, r + int(R[1]) + 6 + 0.5) for r, c in deep.tolist()]
    water = [p for p in cand if g.b[int(p[1]) - 3 : int(p[1]) + 4, int(p[0]) - 3 : int(p[0]) + 4].all()]
    water = rng.sample(water, min(1000, len(water)))
    road = []
    for f in layers["roads"]:
        pr = f["properties"]
        if pr["class"] == "paved" and not pr["bridge"] and pr["width_m"] >= 5.0:
            for x, y in f["geometry"]["coordinates"]:
                if R[0] + 5 < x < R[2] - 5 and R[1] + 5 < y < R[3] - 5:
                    road.append((x, y))
    road = rng.sample(road, min(1000, len(road)))
    blds = []
    for f in layers["buildings"]:
        pr = f["properties"]
        if pr["enterable"] and pr.get("door"):
            ring = f["geometry"]["coordinates"][0]
            cx, cy = ba.centroid(ring)
            d = pr["door"]["center"]
            ox, oy = d[0] + (d[0] - cx) * 0.0, d[1]
            blds.append({"id": pr["_osm_id"], "from": [d[0], _y(h, *d), d[1]], "to": [cx, _y(h, cx, cy), cy]})
    return {"paths": paths, "water": [[x, _y(h, x, y), y] for x, y in water], "road": [[x, _y(h, x, y), y] for x, y in road],
            "buildings": blds}


def run(repo: Path, out: Path, node_dir: Path, tmp: Path) -> int:
    repo, out, node_dir, tmp = repo.resolve(), out.resolve(), node_dir.resolve(), tmp.resolve()
    tmp.mkdir(parents=True, exist_ok=True)
    m1 = out / "m1"
    (m1 / "navmesh_params.json").write_text(json.dumps({**PARAMS, "params_hash": params_hash()}, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    pj = tmp / "params.json"
    pj.write_text(json.dumps(PARAMS), encoding="utf-8")
    rep = {"schema": 1, "params_hash": params_hash()}
    for mode in ("walk", "vehicle"):
        t0 = time.time()
        v, t, st, h = inputs.build(out, mode == "vehicle")
        inputs.write(tmp / f"{mode}.bin", v, t)
        pts = points(out, h)
        (tmp / f"pts_{mode}.json").write_text(json.dumps(pts), encoding="utf-8")
        binp = m1 / ("navmesh.bin" if mode == "walk" else "navmesh_vehicle.bin")
        cmd = ["node", "src/bake.mjs", "--in", str(tmp / f"{mode}.bin"), "--params", str(pj), "--mode", mode, "--out", str(binp),
               "--report", str(tmp / f"r_{mode}.json"), "--params-hash", params_hash(), "--points", str(tmp / f"pts_{mode}.json")]
        r = subprocess.run(cmd, cwd=node_dir, capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stdout, r.stderr)
            return 2
        res = json.loads((tmp / f"r_{mode}.json").read_text())
        res["inputs"] = st
        res["wall_seconds"] = round(time.time() - t0, 1)
        res["bin_file"] = str(binp.relative_to(repo))
        res.pop("config", None)
        rep[mode] = res
    roads = json.loads((out / "roads.geojson").read_text(encoding="utf-8"))["features"]
    narrow = [f["properties"]["_osm_id"] for f in roads if f["properties"]["vehicle_ok"] and f["properties"]["width_m"] < 4.0
              and any(ba.in_rect(x, y) for x, y in f["geometry"]["coordinates"])]
    rep["vehicle"]["roads_narrower_than_4m_excluded"] = sorted(set(narrow))
    rep["connectivity_all_ok"] = all(p["ok"] for m in ("walk", "vehicle") for p in rep[m]["paths"].values())
    (repo / "reports" / "m1-navmesh.json").write_text(json.dumps(rep, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({k: rep[k] for k in ("connectivity_all_ok",)} | {m: {x: rep[m][x] for x in ("tiles", "polys", "bytes", "wall_seconds")} for m in ("walk", "vehicle")}))
    return 0
