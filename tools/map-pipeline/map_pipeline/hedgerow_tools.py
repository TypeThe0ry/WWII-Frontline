"""Hedgerow helpers (M1-033): resampling, gap detection, merge/dedupe, vehicle_blocking."""
from __future__ import annotations

import math

MIN_BLOCK_HEIGHT_M = 1.8
MIN_BLOCK_GAP_M = 4.0
DEDUPE_BUFFER_M = 3.0


def length(pts) -> float:
    return sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:]))


def resample(pts, step: float = 2.0):
    """Even resampling along the polyline (end point kept)."""
    total = length(pts)
    if total == 0 or len(pts) < 2:
        return [tuple(p) for p in pts]
    n = max(1, round(total / step))
    out, seg, acc = [tuple(pts[0])], 0, 0.0
    for k in range(1, n + 1):
        target = total * k / n
        while seg < len(pts) - 2 and acc + math.hypot(pts[seg + 1][0] - pts[seg][0], pts[seg + 1][1] - pts[seg][1]) < target:
            acc += math.hypot(pts[seg + 1][0] - pts[seg][0], pts[seg + 1][1] - pts[seg][1])
            seg += 1
        a, b = pts[seg], pts[seg + 1]
        sl = math.hypot(b[0] - a[0], b[1] - a[1]) or 1.0
        t = min(1.0, max(0.0, (target - acc) / sl))
        out.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
    return out


def chaikin(pts, rounds: int = 1):
    for _ in range(rounds):
        if len(pts) < 3:
            return list(pts)
        new = [pts[0]]
        for a, b in zip(pts, pts[1:]):
            new += [(0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]),
                    (0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1])]
        new.append(pts[-1])
        pts = new
    return pts


def smooth(pts, step: float = 2.0, rounds: int = 2):
    return resample(chaikin(list(pts), rounds), step)


def gaps(lines, max_join_m: float = 12.0):
    """Gaps between endpoints of different lines closer than max_join_m (a continuous hedge
    interrupted by an opening). Returns sorted list of gap widths in metres."""
    ends = [(i, p) for i, ln in enumerate(lines) for p in (ln[0], ln[-1])]
    out = []
    for a in range(len(ends)):
        for b in range(a + 1, len(ends)):
            (i, p), (j, q) = ends[a], ends[b]
            if i == j:
                continue
            d = math.hypot(p[0] - q[0], p[1] - q[1])
            if 0 < d <= max_join_m:
                out.append(round(d, 2))
    return sorted(out)


def vehicle_blocking(height_m: float, max_gap_m: float = 0.0) -> bool:
    """Blocks vehicles when high enough and no gap >= 4 m (MAPS §3.6)."""
    return height_m >= MIN_BLOCK_HEIGHT_M and max_gap_m < MIN_BLOCK_GAP_M


def _pt_seg(p, a, b) -> float:
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = dx * dx + dy * dy
    t = 0.0 if L == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def near(p, line, buf: float) -> bool:
    return any(_pt_seg(p, a, b) <= buf for a, b in zip(line, line[1:]))


def merge(traced, draft, buf: float = DEDUPE_BUFFER_M):
    """Drop OSM-draft lines lying entirely within `buf` of a traced line. -> (kept_draft, dropped)."""
    kept, dropped = [], 0
    for d in draft:
        if any(all(near(p, t, buf) for p in resample(d, 2.0)) for t in traced):
            dropped += 1
        else:
            kept.append(d)
    return kept, dropped
