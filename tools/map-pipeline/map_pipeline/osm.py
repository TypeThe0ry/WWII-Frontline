"""Overpass JSON (out geom) -> cleaned local-metre GeoJSON layers (M1-032)."""
from __future__ import annotations

import gzip
import hashlib
import json
from collections import Counter
from pathlib import Path

from . import clean, geo, modern_filter, osm_rules

LAYERS = ("roads", "water", "hedgerows", "buildings", "forests")
EXIT_MISSING_OSM = 3


class OsmMissing(RuntimeError):
    pass


def find_input(repo: Path):
    """Prefer the raw incoming extract, else the committed gzip copy."""
    for rel in ("assets/incoming/maps/carentan/osm/overpass_carentan.json",
                "data/maps/carentan/source/overpass_carentan.json.gz"):
        p = repo / rel
        if p.is_file():
            return p
    raise OsmMissing("OSM extract not found (assets/incoming/maps/carentan/osm/overpass_carentan.json[.gz])")


def load(path: Path) -> dict:
    raw = path.read_bytes()
    if path.suffix == ".gz":
        raw = gzip.decompress(raw)
    return json.loads(raw)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _local(geom, origin):
    out = []
    for g in geom:
        e, n = geo.to_utm(g["lat"], g["lon"])
        out.append(geo.to_local(e, n, *origin))
    return out


def _join_rings(ways):
    """Chain open member ways into closed rings (point lists, closed)."""
    rings, open_ = [], [list(w) for w in ways if len(w) > 1]
    for w in list(open_):
        if w[0] == w[-1]:
            rings.append(w)
            open_.remove(w)
    while open_:
        cur = open_.pop(0)
        while cur[0] != cur[-1]:
            for i, w in enumerate(open_):
                if w[0] == cur[-1]:
                    cur += w[1:]
                elif w[-1] == cur[-1]:
                    cur += w[::-1][1:]
                elif w[-1] == cur[0]:
                    cur = w + cur[1:]
                elif w[0] == cur[0]:
                    cur = w[::-1] + cur[1:]
                else:
                    continue
                open_.pop(i)
                break
            else:
                break
        if cur[0] == cur[-1] and len(cur) > 3:
            rings.append(cur)
    return rings


def normalise(data: dict, origin) -> list[dict]:
    """Overpass elements -> [{id, tags, lines|polys}] in local metres, sorted by id."""
    res = []
    for e in data["elements"]:
        tags = e.get("tags", {})
        if e["type"] == "way" and "geometry" in e:
            pts = _local(e["geometry"], origin)
            res.append({"id": f"way/{e['id']}", "tags": tags, "pts": pts,
                        "closed": len(pts) > 3 and pts[0] == pts[-1]})
        elif e["type"] == "relation":
            outers = [_local(m["geometry"], origin) for m in e.get("members", [])
                      if m["type"] == "way" and m.get("role") in ("outer", "") and "geometry" in m]
            inners = [_local(m["geometry"], origin) for m in e.get("members", [])
                      if m["type"] == "way" and m.get("role") == "inner" and "geometry" in m]
            o_rings, i_rings = _join_rings(outers), _join_rings(inners)
            if o_rings:
                res.append({"id": f"relation/{e['id']}", "tags": tags, "rings": o_rings, "holes": i_rings})
    res.sort(key=lambda d: d["id"])
    return res


def _feat(props, geom, osm_id):
    p = dict(props)
    p["_osm_id"] = osm_id
    p["verified"] = False
    p["_derived"] = sorted(p.get("_derived", []))
    return {"type": "Feature", "properties": p, "geometry": geom}


def _polys(el):
    if "rings" in el:
        for ring in el["rings"]:
            yield ring, [h for h in el["holes"] if _inside(h[0], ring)]
    elif el["closed"]:
        yield el["pts"], []


def _inside(pt, ring) -> bool:
    x, y = pt
    c = False
    for (x0, y0), (x1, y1) in zip(ring, ring[1:]):
        if (y0 > y) != (y1 > y) and x < (x1 - x0) * (y - y0) / (y1 - y0) + x0:
            c = not c
    return c


def extract(data: dict, origin) -> tuple[dict, dict]:
    """-> ({layer: [features]}, stats)."""
    els = normalise(data, origin)
    kept, dropped = modern_filter.split(els)
    layers = {k: [] for k in LAYERS}
    st = Counter()
    seen = set()

    def add_line(layer, props, pts, oid, key):
        for i, piece in enumerate(clean.clean_lines(pts)):
            k = (layer, key, clean.line_key(piece))
            if k in seen:
                st["duplicates"] += 1
                continue
            seen.add(k)
            layers[layer].append(_feat(props, {"type": "LineString", "coordinates": [list(p) for p in piece]}, oid))

    def add_poly(layer, props, ring, holes, oid):
        rings, why = clean.clean_polygon(ring[:-1] if ring[0] == ring[-1] else ring,
                                         [h[:-1] if h[0] == h[-1] else h for h in holes])
        if rings is None:
            st[f"dropped_{why}"] += 1
            return
        k = (layer, clean.ring_key(rings[0]))
        if k in seen:
            st["duplicates"] += 1
            return
        seen.add(k)
        layers[layer].append(_feat(props, {"type": "Polygon", "coordinates": [[list(p) for p in rg] for rg in rings]}, oid))

    for el in kept:
        t, oid = el["tags"], el["id"]
        if "pts" in el:
            rc = osm_rules.road_class(t)
            if rc is not None:
                extras = {"highway": t.get("highway") or t.get("railway")}
                add_line("roads", {**rc, **extras}, el["pts"], oid, rc["class"])
                continue
        wk = osm_rules.water_kind(t)
        if wk is not None:
            kind, is_area = wk
            props = osm_rules.water_props(t, kind)
            if is_area:
                for ring, holes in _polys(el):
                    add_poly("water", props, ring, holes, oid)
            elif "pts" in el:
                add_line("water", props, el["pts"], oid, kind)
            continue
        if ("pts" in el) and (t.get("barrier") == "hedge" or t.get("natural") == "tree_row"):
            add_line("hedgerows", osm_rules.hedge_props(t), el["pts"], oid, "h")
            continue
        if t.get("barrier") in ("wall", "fence"):
            st["skipped_wall_fence"] += 1
            continue
        if "building" in t:
            props = osm_rules.building_props(t)
            for ring, holes in _polys(el):
                add_poly("buildings", props, ring, holes, oid)
            continue
        fk = osm_rules.forest_kind(t)
        if fk:
            for ring, holes in _polys(el):
                add_poly("forests", osm_rules.forest_props(t, fk), ring, holes, oid)
            continue
        st["unclassified"] += 1
    for k in layers:
        layers[k].sort(key=lambda f: (f["properties"].get("class") or f["properties"].get("kind") or "",
                                      f["properties"]["_osm_id"], json.dumps(f["geometry"]["coordinates"][0][:1])))
    st.update({f"dropped_modern[{k}]": v for k, v in dropped.items()})
    return layers, dict(st)


def dump_layer(path: Path, feats: list) -> str:
    text = json.dumps({"type": "FeatureCollection", "features": feats}, sort_keys=True,
                      separators=(",", ":"), ensure_ascii=False) + "\n"
    path.write_text(text, encoding="utf-8")
    return hashlib.sha256(text.encode()).hexdigest()


def iter_coords(f):
    g = f["geometry"]["coordinates"]
    if f["geometry"]["type"] == "LineString":
        yield from g
    else:
        for ring in g:
            yield from ring


def summary(layers: dict, stats: dict, hashes: dict, meta: dict) -> dict:
    out = {"schema": 1, "osm_timestamp": meta.get("timestamp_osm_base"), "input_sha256": meta.get("sha256"),
           "counts": {k: len(v) for k, v in layers.items()}, "layer_sha256": hashes, "clean": stats}
    roads = layers["roads"]
    out["road_length_m"] = {c: round(sum(clean.length(f["geometry"]["coordinates"]) for f in roads if f["properties"]["class"] == c), 1)
                            for c in sorted({f["properties"]["class"] for f in roads})}
    out["hedgerow_length_m"] = round(sum(clean.length(f["geometry"]["coordinates"]) for f in layers["hedgerows"]), 1)
    out["causeway_candidates"] = sorted({f["properties"]["_osm_id"] for f in roads if f["properties"].get("causeway_candidate")})
    tot = sum(len(f["properties"]["_derived"]) for v in layers.values() for f in v)
    n = sum(len(v) for v in layers.values())
    out["derived_attr_per_feature"] = round(tot / n, 2) if n else 0
    out["out_of_range_vertices"] = sum(1 for v in layers.values() for f in v for x, y in iter_coords(f)
                                       if not (0 <= x <= 2048 and 0 <= y <= 2048))
    s = {"x": (380, 1560), "y": (520, 1700)}
    inside = lambda f: any(s["x"][0] <= x <= s["x"][1] and s["y"][0] <= y <= s["y"][1] for x, y in iter_coords(f))
    out["slice_m1"] = {"buildings": sum(1 for f in layers["buildings"] if inside(f)),
                       "roads": sum(1 for f in roads if inside(f))}
    out["road_connectivity"] = connectivity(roads)
    return out


def connectivity(roads) -> dict:
    """Union-find over vehicle_ok road endpoints/vertices (1 cm snapped)."""
    parent: dict = {}

    def find(a):
        while parent.setdefault(a, a) != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    comp_len: Counter = Counter()
    for f in roads:
        if not f["properties"]["vehicle_ok"]:
            continue
        pts = [tuple(p) for p in f["geometry"]["coordinates"]]
        for a, b in zip(pts, pts[1:]):
            parent[find(a)] = find(b)
    for f in roads:
        if f["properties"]["vehicle_ok"]:
            pts = [tuple(p) for p in f["geometry"]["coordinates"]]
            comp_len[find(pts[0])] += clean.length(pts)
    top = comp_len.most_common(3)
    return {"components": len(comp_len), "largest_m": [round(v, 1) for _, v in top]}
