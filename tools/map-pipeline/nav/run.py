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
    "walk": {**ARCH, "max_edge_len_m": 24.0},
    "vehicle": {"cell_size": 0.5, "cell_height": 0.25, "agent_radius": 2.0, "agent_height": 2.6, "max_climb": 0.6, "max_slope": 25, "max_edge_len_m": 24.0},
    "areas": {"ground": 1, "wade": 2, "door": 3, "water": 0},
    "kcc_walk_slope_limit_deg": 35,
    "note": "walk params follow ARCHITECTURE 16.2 (MAPS 6.2 differs: PI-085); ground=1, wade=2 (cost x2), door=3, water=0 marked in tile bake",
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
            # inside target: 2.5 m from the door towards the centroid (the centroid itself can lie outside a concave footprint)
            L = max(1e-6, ((cx - d[0]) ** 2 + (cy - d[1]) ** 2) ** 0.5)
            k = min(1.0, 2.5 / L)
            ix, iy = d[0] + (cx - d[0]) * k, d[1] + (cy - d[1]) * k
            ox, oy = d[0], d[1]
            blds.append({"id": pr["_osm_id"], "from": [ox, _y(h, ox, oy), oy], "to": [ix, _y(h, ix, iy), iy]})
    return {"paths": paths, "water": [[x, _y(h, x, y), y] for x, y in water], "road": [[x, _y(h, x, y), y] for x, y in road],
            "buildings": blds}


def generate_offmesh(out: Path, h) -> dict:
    """Deterministic initial M1 off-mesh links for enterable building entries (ground-level window/entry
    links; upper-floor stairs await grey-box interiors in M1-052). Both endpoints sit on the walkable
    ground navmesh within <= 0.5 m."""
    blds = json.loads((out / "buildings.geojson").read_text(encoding="utf-8"))["features"]
    links = []
    ent = [f for f in blds if f["properties"].get("enterable") and f["properties"].get("door")]
    ent.sort(key=lambda f: f["properties"]["_osm_id"])
    for idx, f in enumerate(ent, start=1):
        p = f["properties"]
        ring = f["geometry"]["coordinates"][0]
        cx, cy = ba.centroid(ring)
        d = p["door"]
        dx, dz = d["center"]
        if not (ba.RECT[0] + 2 <= dx <= ba.RECT[2] - 2 and ba.RECT[1] + 2 <= dz <= ba.RECT[3] - 2):
            continue
        outward = d.get("outward")
        if outward:
            nx, nz = outward[0], outward[1]
        else:
            L = max(1e-6, ((dx - cx) ** 2 + (dz - cy) ** 2) ** 0.5)
            nx, nz = (dx - cx) / L, (dz - cy) / L
        # start: 1.5 m outside doorway; end: 1.8 m inside towards interior
        sx, sz = round(dx + nx * 1.5, 2), round(dz + nz * 1.5, 2)
        ex, ez = round(dx - nx * 1.8, 2), round(dz - nz * 1.8, 2)
        sy = round(float(h[int(sz), int(sx)]), 2)
        ey = round(float(h[int(ez), int(ex)]), 2)
        links.append({
            "id": f"om_entry_{idx:03d}",
            "kind": "window",
            "building_id": p["_osm_id"],
            "start": [sx, sy, sz],
            "end": [ex, ey, ez],
            "radius": 0.4,
            "bidirectional": True,
            "cost": 2.0,
        })
    return {
        "schema": "frontline.map.offmesh/1",
        "format": "{id, kind: window|ladder|stairs|vault, start:[x,y,z], end:[x,y,z], radius, bidirectional, cost}",
        "offmesh": links,
        "note": "Initial M1 ground-level entry/window links for enterable buildings (>= 80% of doors); multi-floor stairs links await grey-box interiors (M1-052).",
    }


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
        boxes = inputs.area_boxes(out, h, mode == "vehicle")
        areas_p = tmp / f"areas_{mode}.json"
        areas_p.write_text(json.dumps(boxes), encoding="utf-8")
        pts = points(out, h)
        if mode == "vehicle":
            pts.pop("buildings")
        (tmp / f"pts_{mode}.json").write_text(json.dumps(pts), encoding="utf-8")
        offmesh_p = m1 / "offmesh.json"
        if mode == "walk":
            om_doc = generate_offmesh(out, h)
            offmesh_p.write_text(json.dumps(om_doc, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        binp = m1 / ("navmesh.bin" if mode == "walk" else "navmesh_vehicle.bin")
        cmd = ["node", "src/bake.mjs", "--in", str(tmp / f"{mode}.bin"), "--params", str(pj), "--mode", mode, "--out", str(binp),
               "--report", str(tmp / f"r_{mode}.json"), "--params-hash", params_hash(),
               "--points", str(tmp / f"pts_{mode}.json"), "--areas", str(areas_p)]
        if mode == "walk":
            cmd += ["--offmesh", str(offmesh_p)]
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
    print(json.dumps({k: rep[k] for k in ("connectivity_all_ok",)} | {m: {x: rep[m][x] for x in ("tiles", "polys", "bytes", "wall_seconds", "area_polys")} for m in ("walk", "vehicle")}))
    return 0


def query(navmesh_path: Path, from_xz: str, to_xz: str, node_dir: Path) -> int:
    r = subprocess.run(
        ["node", "src/query.mjs", "--in", str(navmesh_path.resolve()), "--from", from_xz, "--to", to_xz],
        cwd=node_dir.resolve(), capture_output=True, text=True,
    )
    if r.stdout:
        print(r.stdout.strip())
    if r.stderr:
        print(r.stderr.strip())
    return r.returncode
