"""Geometry clipping and cleaning for OSM vectors (local metres)."""
from __future__ import annotations

import math

SIZE = 2048.0
MIN_AREA_M2 = 4.0
MIN_LINE_M = 2.0
SNAP_CM = 2  # decimals => 1 cm


def r(v: float) -> float:
    return round(v + 0.0, SNAP_CM)


def clip_line(pts, lo=0.0, hi=SIZE):
    """Clip a polyline to the square [lo,hi]²; returns list of pieces."""
    out, cur = [], []
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        seg = _liang(x0, y0, x1, y1, lo, hi)
        if seg is None:
            if len(cur) > 1:
                out.append(cur)
            cur = []
            continue
        (a, b), (c, d), t0, t1 = seg
        if t0 > 0 and len(cur) > 1:
            out.append(cur)
            cur = []
        if not cur:
            cur = [(a, b)]
        cur.append((c, d))
        if t1 < 1:
            if len(cur) > 1:
                out.append(cur)
            cur = []
    if len(cur) > 1:
        out.append(cur)
    return out


def _liang(x0, y0, x1, y1, lo, hi):
    dx, dy = x1 - x0, y1 - y0
    t0, t1 = 0.0, 1.0
    for p, q in ((-dx, x0 - lo), (dx, hi - x0), (-dy, y0 - lo), (dy, hi - y0)):
        if p == 0:
            if q < 0:
                return None
        else:
            t = q / p
            if p < 0:
                if t > t1:
                    return None
                t0 = max(t0, t)
            else:
                if t < t0:
                    return None
                t1 = min(t1, t)
    if t0 > t1:
        return None
    return (x0 + t0 * dx, y0 + t0 * dy), (x0 + t1 * dx, y0 + t1 * dy), t0, t1


def clip_ring(ring, lo=0.0, hi=SIZE):
    """Sutherland-Hodgman against the square; ring is open (no repeated end)."""
    pts = list(ring)
    for axis, val, keep_ge in ((0, lo, True), (0, hi, False), (1, lo, True), (1, hi, False)):
        if not pts:
            break
        new = []
        prev = pts[-1]
        for cur in pts:
            pin = prev[axis] >= val if keep_ge else prev[axis] <= val
            cin = cur[axis] >= val if keep_ge else cur[axis] <= val
            if cin != pin:
                t = (val - prev[axis]) / (cur[axis] - prev[axis])
                ip = (val, prev[1] + t * (cur[1] - prev[1])) if axis == 0 else (prev[0] + t * (cur[0] - prev[0]), val)
                new.append(ip)
            if cin:
                new.append(cur)
            prev = cur
        pts = new
    return pts


def area(ring) -> float:
    """Signed area (CCW positive) of an open ring."""
    s = 0.0
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        s += x0 * y1 - x1 * y0
    return s / 2.0


def length(pts) -> float:
    return sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:]))


def dedupe_points(pts, closed=False):
    out = []
    for p in pts:
        p = (r(p[0]), r(p[1]))
        if not out or out[-1] != p:
            out.append(p)
    if closed and len(out) > 1 and out[0] == out[-1]:
        out.pop()
    return out


def _seg_x(p1, p2, p3, p4) -> bool:
    def o(a, b, c):
        return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])

    d1, d2, d3, d4 = o(p3, p4, p1), o(p3, p4, p2), o(p1, p2, p3), o(p1, p2, p4)
    return ((d1 > 0) != (d2 > 0)) and ((d3 > 0) != (d4 > 0)) and d1 * d2 != 0 and d3 * d4 != 0


def self_intersects(ring) -> bool:
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        for j in range(i + 2, n):
            if i == 0 and j == n - 1:
                continue
            if _seg_x(a, b, ring[j], ring[(j + 1) % n]):
                return True
    return False


def _bbox_inside(ring, lo=0.0, hi=SIZE):
    return all(lo <= x <= hi and lo <= y <= hi for x, y in ring)


def clean_polygon(outer, holes=()):
    """-> (rings|None, reason). rings[0] CCW outer, rest CW holes (closed)."""
    stats = {"self_intersect": 0, "small": 0}
    o = dedupe_points(clip_ring(outer) if not _bbox_inside(outer) else outer, closed=True)
    if len(o) < 3:
        return None, "outside_box" if not _bbox_inside(outer) else "degenerate"
    if self_intersects(o):
        return None, "self_intersect"
    if abs(area(o)) < MIN_AREA_M2:
        return None, "small"
    if area(o) < 0:
        o.reverse()
    rings = [o + [o[0]]]
    for h in holes:
        hh = dedupe_points(clip_ring(h) if not _bbox_inside(h) else h, closed=True)
        if len(hh) < 3 or abs(area(hh)) < MIN_AREA_M2 or self_intersects(hh):
            continue
        if area(hh) > 0:
            hh.reverse()
        rings.append(hh + [hh[0]])
    return rings, None


def clean_lines(pts):
    """Clip + dedupe a polyline -> list of cleaned pieces (>= MIN_LINE_M)."""
    res = []
    for piece in clip_line(pts):
        p = dedupe_points(piece)
        if len(p) >= 2 and length(p) >= MIN_LINE_M:
            res.append(p)
    return res


def line_key(p):
    t = tuple(p)
    return min(t, t[::-1])


def ring_key(ring):
    t = tuple(ring[:-1])
    i = t.index(min(t))
    t = t[i:] + t[:i]
    return t


def extend_dangles(road_features: list[dict], max_gap_m: float = 1.5) -> int:
    """Extend/snap dangling endpoints of vehicle_ok road LineStrings onto the nearest vertex of
    another vehicle_ok road within ``max_gap_m`` (0 < d <= max_gap_m), closing tiny OSM digitising
    gaps deterministically without creating out-of-bounds coordinates."""
    from collections import Counter

    deg: Counter = Counter()
    veh_idx = []
    for i, f in enumerate(road_features):
        if not f["properties"].get("vehicle_ok"):
            continue
        pts = f["geometry"]["coordinates"]
        if len(pts) < 2:
            continue
        veh_idx.append(i)
        for p in pts:
            deg[(p[0], p[1])] += 1

    # All candidate target vertices from vehicle_ok roads (sorted for determinism)
    all_verts = []
    for i in veh_idx:
        for p in road_features[i]["geometry"]["coordinates"]:
            all_verts.append((p[0], p[1], i))
    all_verts.sort()

    extended = 0
    for i in veh_idx:
        coords = road_features[i]["geometry"]["coordinates"]
        for end_idx in (0, -1):
            ep = (coords[end_idx][0], coords[end_idx][1])
            if deg[ep] > 1:
                continue  # already connected to another vertex/road
            # Skip boundary endpoints clipped on the map edge
            if ep[0] <= 0.01 or ep[0] >= SIZE - 0.01 or ep[1] <= 0.01 or ep[1] >= SIZE - 0.01:
                continue
            best = None
            for vx, vy, j in all_verts:
                if j == i:
                    continue
                d = math.hypot(vx - ep[0], vy - ep[1])
                if 0.0 < d <= max_gap_m:
                    cand = (round(d, 6), vx, vy)
                    if best is None or cand < best:
                        best = cand
            if best is not None:
                _, tx, ty = best
                if 0.0 <= tx <= SIZE and 0.0 <= ty <= SIZE:
                    if end_idx == 0:
                        coords.insert(0, [r(tx), r(ty)])
                    else:
                        coords.append([r(tx), r(ty)])
                    deg[(r(tx), r(ty))] += 1
                    deg[ep] += 1
                    extended += 1
    return extended
