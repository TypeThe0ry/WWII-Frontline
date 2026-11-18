"""Building annotation and core landmarks for the M1 slice (M1-034)."""
from __future__ import annotations

import math

from . import clean

RECT = (380.0, 520.0, 1560.0, 1700.0)
ENTERABLE_MAX_FRACTION = 0.25
ENTERABLE_MIN_AREA_M2 = 60.0
ENTERABLE_NEAR_ZONE_M = 60.0
DOOR_WIDTH_M = 1.2
LEVEL_HEIGHT_M = 3.0


def in_rect(x, y, rect=RECT) -> bool:
    return rect[0] <= x <= rect[2] and rect[1] <= y <= rect[3]


def centroid(ring):
    pts = ring[:-1]
    return sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts)


def bbox(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return min(xs), min(ys), max(xs), max(ys)


def style_of(area, elong, zone_tag, btype) -> str:
    if btype == "church":
        return "church"
    if zone_tag == "farm" and area >= 80:
        return "farm_u_court" if elong < 2.5 else "barn"
    if zone_tag == "centre":
        return "stone_townhouse"
    return "norman_house" if area < 200 else "stone_hall"


def levels_of(area, zone_tag, derived_levels, was_derived) -> int:
    if not was_derived:
        return derived_levels
    if zone_tag == "centre":
        return 3 if area >= 120 else 2
    return 2 if area >= 90 else 1


def zone_tag(x, y, zones) -> str:
    """'centre' near L3, 'farm' near L1a, else 'suburb'."""
    for z in zones:
        d = math.hypot(x - z["pos"][0], y - z["pos"][1])
        if z["id"] == "L3a" and d <= 300:
            return "centre"
        if z["id"] == "L1a" and d <= 150:
            return "farm"
    return "suburb"


def annotate(buildings: list, zones: list):
    """Mutates properties in place. Returns stats dict. Only buildings inside the slice rect
    are annotated; enterable <= 25% of them (largest near active zones first)."""
    cands = []
    inside = []
    for f in buildings:
        ring = f["geometry"]["coordinates"][0]
        cx, cy = centroid(ring)
        if not in_rect(cx, cy):
            continue
        a = abs(clean.area(ring[:-1]))
        x0, y0, x1, y1 = bbox(ring)
        w, h = sorted((x1 - x0, y1 - y0))
        p = f["properties"]
        tag = zone_tag(cx, cy, zones)
        p["style"] = style_of(a, (h / w) if w else 9.0, tag, p.get("type"))
        p["levels"] = levels_of(a, tag, p["levels"], "levels" in p["_derived"])
        p["destructible"] = False
        p["enterable"] = False
        p["asset_ref"] = "V-BLD-PLACEHOLDER"
        p["_derived"] = sorted(set(p["_derived"]) | {"style", "asset_ref"})
        inside.append(f)
        near = min((math.hypot(cx - z["pos"][0], cy - z["pos"][1]) - z["radius_m"] for z in zones), default=1e9)
        if a >= ENTERABLE_MIN_AREA_M2 and near <= ENTERABLE_NEAR_ZONE_M:
            cands.append((near, -a, f["properties"]["_osm_id"], f))
    cap = int(len(inside) * ENTERABLE_MAX_FRACTION)
    cands.sort(key=lambda t: t[:3])
    for _, _, _, f in cands[:cap]:
        f["properties"]["enterable"] = True
    return {"in_slice": len(inside), "enterable": sum(1 for f in inside if f["properties"]["enterable"]),
            "enterable_cap": cap}


def door_for(ring, road_lines):
    """Door centre on the footprint edge midpoint closest to a road (or the longest edge)."""
    best = None
    pts = ring[:-1]
    for a, b in zip(pts, pts[1:] + pts[:1]):
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        ln = math.hypot(b[0] - a[0], b[1] - a[1])
        if ln < DOOR_WIDTH_M + 0.6:
            continue
        d = min((math.hypot(mid[0] - p[0], mid[1] - p[1]) for r in road_lines for p in r), default=0.0)
        key = (d, -ln)
        if best is None or key < best[0]:
            best = (key, mid)
    return None if best is None else {"center": [clean.r(best[1][0]), clean.r(best[1][1])], "width_m": DOOR_WIDTH_M}


def landmark_boxes(buildings, anchors):
    """anchors: [(id, name, kind, design_pos, fidelity, asset_ref, levels_h_m, search_m)].
    Picks the footprint nearest each anchor (OSM position wins over design)."""
    out = []
    for lid, name, kind, pos, fid, ref, height, rad in anchors:
        best = None
        for f in buildings:
            ring = f["geometry"]["coordinates"][0]
            cx, cy = centroid(ring)
            d = math.hypot(cx - pos[0], cy - pos[1])
            if d <= rad and (best is None or d < best[0]):
                best = (d, f, (cx, cy), ring)
        if best is None:
            out.append({"id": lid, "name": name, "kind": kind, "design": pos, "found_in_osm": False,
                        "fidelity": fid, "asset_ref": ref})
            continue
        d, f, (cx, cy), ring = best
        x0, y0, x1, y1 = bbox(ring)
        out.append({"id": lid, "name": name, "kind": kind, "design": pos, "found_in_osm": True,
                    "pos_e": clean.r(cx), "pos_n": clean.r(cy), "offset_from_design_m": round(d, 1),
                    "osm_id": f["properties"]["_osm_id"], "fidelity": fid, "asset_ref": ref,
                    "placeholder_dims_m": {"l": round(x1 - x0, 1), "w": round(y1 - y0, 1), "h": height},
                    "floors": [{"level": i, "z_m": i * LEVEL_HEIGHT_M} for i in range(max(1, int(height // LEVEL_HEIGHT_M)))],
                    "doors": [], "state_1944": "intact"})
    return out
