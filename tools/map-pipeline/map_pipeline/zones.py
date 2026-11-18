"""M1 slice zones, spawns, routes, cover points and the rule validator (M1-034)."""
from __future__ import annotations

import heapq
import json
import math
from collections import deque
from pathlib import Path

import numpy as np

from . import apply_edits, buildings_annotate as ba, clean

SCHEMA = "frontline.map.zones/1"
RECT = ba.RECT
OOB_BUFFER_M = 64
SNAP_MAX_M = 60.0
SECTORS = 12
MIN_ENTRANCES = 3
MIN_COVER = 8
VEHICLE_MIN_WIDTH_M = 3.6  # Sherman 2.62 m + 1 m
HQ_RADIUS_M = 40
# design values (MAPS §3.6 / §3.8)
POINTS = [
    ("L1a", "carentan.ingouf_farm", "R", (900, 1450), 250, 45, ["stone_courtyard", "hedgerow"]),
    ("L1b", "carentan.southern_bridge", "R", (560, 1700), 150, 35, ["bridge", "chokepoint"]),
    ("L2a", "carentan.market_garden_fields", "G", (700, 800), 80, 55, ["hedgerow_grid"]),
    ("L2c", "carentan.orchard_cross", "G", (500, 1000), 80, 50, ["hedgerow_gap"]),
    ("L3a", "carentan.notre_dame", "R", (1277, 897), 40, 50, ["church", "graveyard_wall"]),
    ("L3b", "carentan.place_de_la_republique", "R", (1380, 990), 120, 45, ["arcade"]),
]
ACTIVE = ["L1a", "L2a", "L3a"]
HQ = {"US": (480, 1600), "GER": (1480, 560)}
GARRISON = {"US": (650, 1470), "GER": (1340, 720)}


class Grid:
    """2 m occupancy grid over the whole map; `blocked[row=y, col=x]`."""

    def __init__(self, blocked: np.ndarray, res: float = 2.0):
        self.b, self.res = blocked, res
        self.n = blocked.shape[0]

    def cell(self, x, y):
        return int(y // self.res), int(x // self.res)

    def standable(self, x, y) -> bool:
        r, c = self.cell(x, y)
        return 0 <= r < self.n and 0 <= c < self.n and not self.b[r, c]

    def snap(self, x, y, max_m=SNAP_MAX_M):
        """Nearest standable point (ring search); returns ((x, y), distance) or None."""
        if self.standable(x, y):
            return (x, y), 0.0
        best = None
        k = int(max_m / self.res)
        r0, c0 = self.cell(x, y)
        for dr in range(-k, k + 1):
            for dc in range(-k, k + 1):
                r, c = r0 + dr, c0 + dc
                if 0 <= r < self.n and 0 <= c < self.n and not self.b[r, c]:
                    px, py = (c + 0.5) * self.res, (r + 0.5) * self.res
                    d = math.hypot(px - x, py - y)
                    if d <= max_m and (best is None or (d, px, py) < best[0]):
                        best = ((d, px, py), (px, py))
        return None if best is None else (best[1], round(best[0][0], 2))


def build_grid(buildings, water, res=2.0) -> Grid:
    n = int(2048 / res)
    blocked = np.zeros((n, n), dtype=bool)
    for f in buildings:
        ring = [tuple(p) for p in f["geometry"]["coordinates"][0][:-1]]
        _fill(blocked, ring, res)
    for f in water:
        p = f["properties"]
        if p.get("walkable_depth_m", 0.5) > 0.5 and f["geometry"]["type"] == "Polygon" and p["kind"] != "flood_marsh":
            _fill(blocked, [tuple(q) for q in f["geometry"]["coordinates"][0][:-1]], res)
        elif f["geometry"]["type"] == "LineString" and p["kind"] in ("river", "canal"):
            _buffer_line(blocked, f["geometry"]["coordinates"], p.get("width_m", 10) / 2.0, res)
        elif f["geometry"]["type"] == "Polygon" and p["kind"] == "pond":
            _fill(blocked, [tuple(q) for q in f["geometry"]["coordinates"][0][:-1]], res)
    return Grid(blocked, res)


def _fill(blocked, ring, res):
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    c0, c1 = max(0, int(min(xs) // res)), min(blocked.shape[1] - 1, int(max(xs) // res))
    r0, r1 = max(0, int(min(ys) // res)), min(blocked.shape[0] - 1, int(max(ys) // res))
    if c1 < c0 or r1 < r0:
        return
    X, Y = np.meshgrid((np.arange(c0, c1 + 1) + 0.5) * res, (np.arange(r0, r1 + 1) + 0.5) * res)
    blocked[r0 : r1 + 1, c0 : c1 + 1] |= apply_edits._inside(X, Y, ring)


def _buffer_line(blocked, pts, half, res):
    for a, b in zip(pts, pts[1:]):
        c0 = max(0, int((min(a[0], b[0]) - half) // res))
        c1 = min(blocked.shape[1] - 1, int((max(a[0], b[0]) + half) // res))
        r0 = max(0, int((min(a[1], b[1]) - half) // res))
        r1 = min(blocked.shape[0] - 1, int((max(a[1], b[1]) + half) // res))
        if c1 < c0 or r1 < r0:
            continue
        X, Y = np.meshgrid((np.arange(c0, c1 + 1) + 0.5) * res, (np.arange(r0, r1 + 1) + 0.5) * res)
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = dx * dx + dy * dy
        t = np.clip(((X - a[0]) * dx + (Y - a[1]) * dy) / L, 0, 1) if L else np.zeros_like(X)
        d = np.hypot(X - a[0] - t * dx, Y - a[1] - t * dy)
        blocked[r0 : r1 + 1, c0 : c1 + 1] |= d <= half


def snap_open(grid: Grid, pos, radius, min_sectors=MIN_ENTRANCES, max_m=SNAP_MAX_M, step=4.0):
    """Nearest standable point (<= max_m) whose disc has >= min_sectors openings; None if none."""
    cands = []
    k = int(max_m / step)
    for dx in range(-k, k + 1):
        for dy in range(-k, k + 1):
            x, y = pos[0] + dx * step, pos[1] + dy * step
            d = math.hypot(x - pos[0], y - pos[1])
            if d <= max_m and grid.standable(x, y):
                cands.append((d, x, y))
    for d, x, y in sorted(cands):
        if rule_entrances(grid, (x, y), radius) >= min_sectors:
            return (x, y), round(d, 2)
    return None


# ---------------------------------------------------------------- rules (pure, testable)
def rule_not_blocked(grid: Grid, pos) -> bool:
    return grid.standable(*pos)


def rule_entrances(grid: Grid, pos, radius, sectors=SECTORS) -> int:
    """Number of distinct angular sectors through which the zone centre can be walked out of: BFS
    from the centre (8-neighbour, standable cells) inside the disc R only (so walking round the outside does not count); cells reached within
    2 cells of the rim are binned into `sectors` sectors."""
    s = grid.cell(*pos)
    if not grid.standable(*pos):
        return 0
    lim = radius
    seen = {s}
    dq = deque([s])
    hit = set()
    while dq:
        cur = dq.popleft()
        x, y = (cur[1] + 0.5) * grid.res, (cur[0] + 0.5) * grid.res
        d = math.hypot(x - pos[0], y - pos[1])
        if d >= radius - 2 * grid.res:
            hit.add(int(((math.atan2(y - pos[1], x - pos[0]) % (2 * math.pi)) / (2 * math.pi)) * sectors) % sectors)
        for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx = (cur[0] + dr, cur[1] + dc)
            if nx in seen or not (0 <= nx[0] < grid.n and 0 <= nx[1] < grid.n) or grid.b[nx]:
                continue
            if math.hypot((nx[1] + 0.5) * grid.res - pos[0], (nx[0] + 0.5) * grid.res - pos[1]) > lim:
                continue
            seen.add(nx)
            dq.append(nx)
    return len(hit)


def infantry_path(grid: Grid, a, b, rect=RECT, margin=100.0):
    """8-neighbour BFS on the grid inside rect(+margin). Returns list of points or None."""
    lo = (rect[0] - margin, rect[1] - margin)
    hi = (rect[2] + margin, rect[3] + margin)
    s, t = grid.cell(*a), grid.cell(*b)
    if grid.b[s] or grid.b[t]:
        return None
    r0, c0 = grid.cell(*lo)
    r1, c1 = grid.cell(*hi)
    prev = {s: None}
    dq = deque([s])
    while dq:
        cur = dq.popleft()
        if cur == t:
            break
        for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)):
            nx = (cur[0] + dr, cur[1] + dc)
            if nx in prev or not (r0 <= nx[0] <= r1 and c0 <= nx[1] <= c1) or grid.b[nx]:
                continue
            if dr and dc and (grid.b[cur[0] + dr, cur[1]] or grid.b[cur[0], cur[1] + dc]):
                continue
            prev[nx] = cur
            dq.append(nx)
    if t not in prev:
        return None
    path, cur = [], t
    while cur is not None:
        path.append(((cur[1] + 0.5) * grid.res, (cur[0] + 0.5) * grid.res))
        cur = prev[cur]
    return path[::-1]


class RoadGraph:
    def __init__(self, roads):
        self.adj: dict = {}
        for f in roads:
            p = f["properties"]
            if not p["vehicle_ok"]:
                continue
            pts = [tuple(c) for c in f["geometry"]["coordinates"]]
            for a, b in zip(pts, pts[1:]):
                w = math.hypot(b[0] - a[0], b[1] - a[1])
                meta = (p["width_m"], p["bridge"], p["_osm_id"])
                self.adj.setdefault(a, []).append((b, w, meta))
                self.adj.setdefault(b, []).append((a, w, meta))

    def nearest(self, pos, max_m=80.0):
        best = min(self.adj, key=lambda n: (math.hypot(n[0] - pos[0], n[1] - pos[1]), n)) if self.adj else None
        if best is None or math.hypot(best[0] - pos[0], best[1] - pos[1]) > max_m:
            return None
        return best

    def path(self, a, b):
        dist, prev = {a: 0.0}, {a: None}
        pq = [(0.0, a)]
        while pq:
            d, u = heapq.heappop(pq)
            if u == b:
                break
            if d > dist.get(u, 1e18):
                continue
            for v, w, meta in self.adj.get(u, []):
                nd = d + w
                if nd < dist.get(v, 1e18):
                    dist[v], prev[v] = nd, (u, meta)
                    heapq.heappush(pq, (nd, v))
        if b not in prev:
            return None
        pts, metas, cur = [b], [], b
        while prev[cur]:
            cur, m = prev[cur]
            pts.append(cur)
            metas.append(m)
        return pts[::-1], metas[::-1], dist[b]


def route_summary(graph: RoadGraph, stops):
    """stops: ordered positions. -> dict with polyline, min width, chokepoints, length."""
    nodes = [graph.nearest(p) for p in stops]
    if any(n is None for n in nodes):
        return None
    poly, metas, total = [], [], 0.0
    for a, b in zip(nodes, nodes[1:]):
        r = graph.path(a, b)
        if r is None:
            return None
        poly += r[0] if not poly else r[0][1:]
        metas += r[1]
        total += r[2]
    minw = min((m[0] for m in metas), default=0.0)
    choke = sorted({m[2] for m in metas if m[1] or m[0] < VEHICLE_MIN_WIDTH_M + 1.0})
    return {"polyline": [[clean.r(x), clean.r(y)] for x, y in poly], "length_m": round(total, 1),
            "min_width_m": minw, "chokepoints": choke}


def cover_points(grid: Grid, centre, radius, buildings, hedges, want=16):
    cands = []
    lim = radius + 30.0
    for f in buildings:
        ring = f["geometry"]["coordinates"][0]
        cx, cy = ba.centroid(ring)
        if math.hypot(cx - centre[0], cy - centre[1]) > lim + 20:
            continue
        for a, b in zip(ring, ring[1:]):
            mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
            ln = math.hypot(b[0] - a[0], b[1] - a[1]) or 1.0
            for s in (1, -1):
                nx, ny = s * (b[1] - a[1]) / ln * 1.5, s * -(b[0] - a[0]) / ln * 1.5
                p = (mx + nx, my + ny)
                if grid.standable(*p) and math.hypot(p[0] - centre[0], p[1] - centre[1]) <= lim:
                    cands.append((math.hypot(p[0] - centre[0], p[1] - centre[1]), p, "building"))
    for h in hedges:
        for x, y in h["geometry"]["coordinates"]:
            if math.hypot(x - centre[0], y - centre[1]) <= lim and grid.standable(x, y):
                cands.append((math.hypot(x - centre[0], y - centre[1]), (x, y), "hedge"))
    cands.sort(key=lambda t: (round(t[0], 2), t[1]))
    out = []
    for d, p, kind in cands:
        if all(math.hypot(p[0] - q["pos"][0], p[1] - q["pos"][1]) >= 5.0 for q in out):
            out.append({"pos": [clean.r(p[0]), clean.r(p[1])], "kind": kind})
            if len(out) >= want:
                break
    if len(out) < MIN_COVER:  # no mapped cover: clearly-labelled placeholders, never passed off as survey data
        k = 0
        while len(out) < MIN_COVER:
            ang = 2 * math.pi * k / MIN_COVER
            for rr in (radius * 0.8, radius * 1.2, radius * 0.5):
                p = (centre[0] + rr * math.cos(ang), centre[1] + rr * math.sin(ang))
                if grid.standable(*p) and len(out) < MIN_COVER:
                    out.append({"pos": [clean.r(p[0]), clean.r(p[1])], "kind": "placeholder_ring"})
                    break
            k += 1
            if k > 64:
                break
    return out


# ---------------------------------------------------------------- driver
def _load(p: Path):
    return json.loads(p.read_text(encoding="utf-8"))


def _dump(p: Path, obj):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj, sort_keys=True, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")


def build(repo: Path, out: Path) -> dict:
    """Generate m1/{zones,spawns,vehicle_routes,cover_points,nav_critical}.json, landmarks.json and
    annotate buildings.geojson. Similarity is not applied (design == local frame, PI-081)."""
    layers = {k: _load(out / f"{k}.geojson") for k in ("buildings", "roads", "water", "hedgerows")}
    grid = build_grid(layers["buildings"]["features"], layers["water"]["features"])
    notes, lines = [], {}
    for pid, key, org, pos, err, rad, tags in POINTS:
        sn = grid.snap(*pos)
        if sn is None:
            raise SystemExit(f"{pid}: no standable cell within {SNAP_MAX_M} m of {pos}")
        (px, py), d = sn
        if d > 0:
            notes.append(f"{pid}: design {list(pos)} not standable; snapped {d:.1f} m to [{px:.1f},{py:.1f}]")
        lines.setdefault(pid[:2], []).append({"id": pid, "name_key": key, "origin": org, "pos": [clean.r(px), clean.r(py)],
                                              "design_pos": list(pos), "pos_error_m": err, "radius_m": rad, "tags": tags,
                                              "snap_m": d})
    allpts = [p for v in lines.values() for p in v]
    zones = {"schema": SCHEMA, "lines": [{"id": k, "points": v} for k, v in sorted(lines.items())],
             "m1_active": ACTIVE, "playable_polygon": [[RECT[0], RECT[1]], [RECT[2], RECT[1]], [RECT[2], RECT[3]], [RECT[0], RECT[3]]],
             "out_of_bounds_buffer_m": OOB_BUFFER_M, "notes": notes}
    hq, gar = {}, {}
    for side, pos in HQ.items():
        sn = snap_open(grid, pos, HQ_RADIUS_M)
        if sn is None:
            raise SystemExit(f"HQ {side}: no open position within {SNAP_MAX_M} m of {pos}")
        (px, py), d = sn
        hq[side] = {"pos": [clean.r(px), clean.r(py)], "design_pos": list(pos), "radius_m": HQ_RADIUS_M, "snap_m": d}
        if d:
            notes.append(f"HQ {side}: design {list(pos)} snapped {d:.1f} m")
    for side, pos in GARRISON.items():
        (px, py), d = grid.snap(*pos)
        gar[side] = {"pos": [clean.r(px), clean.r(py)], "design_pos": list(pos), "snap_m": d,
                     "hq_distance_m": round(math.hypot(px - hq[side]["pos"][0], py - hq[side]["pos"][1]), 1)}
    spawns = {"schema": "frontline.map.spawns/1", "hq": hq, "garrison_candidates": gar, "kinds_supported": ["hq", "garrison"],
              "cover_zones": [p["id"] for p in allpts if p["id"] in ACTIVE], "note": "M1 slice only; not valid for the full map"}
    # buildings
    act = [p for p in allpts if p["id"] in ACTIVE]
    stats = ba.annotate(layers["buildings"]["features"], act)
    roadpts = [f["geometry"]["coordinates"] for f in layers["roads"]["features"] if f["properties"]["vehicle_ok"]]
    # landmarks
    bs = layers["buildings"]["features"]
    church = [f for f in bs if f["properties"].get("type") == "church"]
    lm = ba.landmark_boxes(church, [("church_notre_dame", "Notre-Dame de Carentan", "church", (1277, 897), "A", "V-BLD-CHURCH", 28.0, 60)])
    lm += ba.landmark_boxes(bs, [("farm_ingouf", "Ferme Ingouf", "farm", (900, 1450), "B", "V-BLD-FARM", 7.0, 80),
                                 ("arcade_square", "Place de la République (unverified)", "arcade", (1380, 990), "B", "V-BLD-ARCADE", 9.0, 120)])
    bridges = [f for f in layers["roads"]["features"] if f["properties"]["bridge"] and f["properties"]["class"] != "rail"]
    bpos = None
    for f in bridges:
        c = f["geometry"]["coordinates"]
        m = c[len(c) // 2]
        if math.hypot(m[0] - 560, m[1] - 1700) <= 200 and (bpos is None or math.hypot(m[0] - 560, m[1] - 1700) < bpos[0]):
            bpos = (math.hypot(m[0] - 560, m[1] - 1700), m, f)
    lm.append({"id": "bridge_south", "name": "Southern Bridge (L1b)", "kind": "bridge", "design": [560, 1700], "fidelity": "A",
               "asset_ref": "V-BLD-BRIDGE", "found_in_osm": bpos is not None,
               **({"pos_e": bpos[1][0], "pos_n": bpos[1][1], "osm_id": bpos[2]["properties"]["_osm_id"],
                   "placeholder_dims_m": {"l": round(clean.length(bpos[2]["geometry"]["coordinates"]), 1),
                                          "w": bpos[2]["properties"]["width_m"], "h": 2.0}} if bpos else {})})
    byid = {f["properties"]["_osm_id"]: f for f in bs}
    for m in lm:
        if m.get("osm_id") in byid and m["kind"] != "bridge":
            d = ba.door_for(byid[m["osm_id"]]["geometry"]["coordinates"][0], roadpts)
            m["doors"] = [d] if d else []
    # doors for enterable buildings are recorded in the annotation
    for f in bs:
        if f["properties"]["enterable"]:
            d = ba.door_for(f["geometry"]["coordinates"][0], roadpts)
            f["properties"]["door"] = d
    # routes
    rg = RoadGraph(layers["roads"]["features"])
    route = route_summary(rg, [hq["US"]["pos"], [p for p in act if p["id"] == "L1a"][0]["pos"],
                               [p for p in act if p["id"] == "L3a"][0]["pos"]])
    ger = route_summary(rg, [hq["US"]["pos"], hq["GER"]["pos"]])
    routes = {"schema": "frontline.map.vehicle_routes/1", "m1_main_route": route,
              "us_to_ger_hq": None if ger is None else {k: ger[k] for k in ("length_m", "min_width_m", "chokepoints")},
              "blocking": {"flood_marsh": [f["properties"]["_osm_id"] for f in layers["water"]["features"] if f["properties"]["kind"] == "flood_marsh"],
                           "hedge_blocking_segments": sum(1 for f in layers["hedgerows"]["features"] if f["properties"]["vehicle_blocking"])},
              "note": "N13 causeway lies outside the box (PI-083); route starts at the road node nearest to the US HQ"}
    cover = {p["id"]: cover_points(grid, p["pos"], p["radius_m"], bs, layers["hedgerows"]["features"]) for p in act}
    path = infantry_path(grid, hq["US"]["pos"], hq["GER"]["pos"])
    nav = {"schema": "frontline.map.nav_critical/1",
           "hq_to_hq_infantry": None if path is None else [[clean.r(x), clean.r(y)] for x, y in path[::10]],
           "hq_open_sectors": {s: rule_entrances(grid, tuple(v["pos"]), HQ_RADIUS_M) for s, v in hq.items()}}
    m1 = out / "m1"
    _dump(m1 / "zones.json", zones)
    _dump(m1 / "spawns.json", spawns)
    _dump(m1 / "cover_points.json", cover)
    _dump(m1 / "nav_critical.json", nav)
    _dump(out / "vehicle_routes.json", routes)
    _dump(out / "landmarks.json", {"schema": "frontline.map.landmarks/1", "landmarks": lm})
    from . import osm
    osm.dump_layer(out / "buildings.geojson", layers["buildings"]["features"])
    return {"annotate": stats, "notes": notes}


def validate_slice(repo: Path, out: Path):
    """Run all M1-034 rules on the written data. Returns (report, ok)."""
    layers = {k: _load(out / f"{k}.geojson") for k in ("buildings", "roads", "water")}
    z = _load(out / "m1" / "zones.json")
    sp = _load(out / "m1" / "spawns.json")
    cov = _load(out / "m1" / "cover_points.json")
    lm = _load(out / "landmarks.json")["landmarks"]
    vr = _load(out / "vehicle_routes.json")
    grid = build_grid(layers["buildings"]["features"], layers["water"]["features"])
    pts = {p["id"]: p for line in z["lines"] for p in line["points"]}
    rules = {}

    def rule(name, ok, detail):
        rules[name] = {"pass": bool(ok), "detail": detail}

    rule("playable_polygon", len(z["playable_polygon"]) >= 4 and z["out_of_bounds_buffer_m"] == OOB_BUFFER_M, z["out_of_bounds_buffer_m"])
    bad = [i for i in z["m1_active"] if not rule_not_blocked(grid, pts[i]["pos"])]
    rule("zone_not_in_water_or_building", not bad, bad or "ok")
    ent = {i: rule_entrances(grid, pts[i]["pos"], pts[i]["radius_m"]) for i in z["m1_active"]}
    rule("zone_entrances>=3", all(v >= MIN_ENTRANCES for v in ent.values()), ent)
    snaps = {i: p["snap_m"] for i, p in pts.items() if p["snap_m"]}
    rule("snap<=60m", all(v <= SNAP_MAX_M for v in snaps.values()), snaps)
    hq = {s: tuple(v["pos"]) for s, v in sp["hq"].items()}
    path = infantry_path(grid, hq["US"], hq["GER"])
    rule("hq_infantry_connected", path is not None, None if path is None else round(len(path) * grid.res, 1))
    rg = RoadGraph(layers["roads"]["features"])
    a, b = rg.nearest(hq["US"]), rg.nearest(hq["GER"])
    vp = rg.path(a, b) if a and b else None
    rule("hq_vehicle_connected", vp is not None, None if vp is None else round(vp[2], 1))
    main = vr["m1_main_route"]
    rule("vehicle_route_width>=3.6", main is not None and main["min_width_m"] >= VEHICLE_MIN_WIDTH_M, None if main is None else main["min_width_m"])
    rule("vehicle_route_chokepoint>=1", main is not None and len(main["chokepoints"]) >= 1, None if main is None else main["chokepoints"])
    bs = [f for f in layers["buildings"]["features"] if ba.in_rect(*ba.centroid(f["geometry"]["coordinates"][0]))]
    en = [f for f in bs if f["properties"]["enterable"]]
    rule("enterable<=25%", len(en) <= ba.ENTERABLE_MAX_FRACTION * len(bs), {"enterable": len(en), "total": len(bs)})
    rule("enterable_have_door", all(f["properties"].get("door") and f["properties"]["levels"] >= 1 for f in en), sum(1 for f in en if not f["properties"].get("door")))
    need = {"church_notre_dame", "farm_ingouf", "arcade_square", "bridge_south"}
    have = {m["id"]: m for m in lm}
    rule("landmarks_defined", need <= set(have) and all(have[i].get("asset_ref") for i in need), sorted(need - set(have)))
    rule("landmark_church_found_in_osm", have["church_notre_dame"]["found_in_osm"], {i: have[i]["found_in_osm"] for i in sorted(need) if i in have})
    rule("cover_points>=8", all(len(v) >= MIN_COVER for v in cov.values()),
         {k: {"total": len(v), "mapped": sum(1 for c in v if c["kind"] != "placeholder_ring")} for k, v in cov.items()})
    rule("nav_not_enclosed", all(v >= MIN_ENTRANCES for v in _load(out / "m1" / "nav_critical.json")["hq_open_sectors"].values()),
         _load(out / "m1" / "nav_critical.json")["hq_open_sectors"])
    leak = []
    for fn in ("zones.json", "spawns.json"):
        p = out / fn
        if p.is_file() and any(s in p.read_text(encoding="utf-8") for s in ("[480,1600]", "[1480,560]", "[420,1550]", "[1500,600]")):
            leak.append(fn)
    rule("no_m1_coords_in_full_map_files", not leak, leak or "ok")
    ok = all(r["pass"] for r in rules.values())
    return {"schema": 1, "ok": ok, "rules": rules}, ok
