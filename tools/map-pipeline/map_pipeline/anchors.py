"""Anchor -> similarity transform (MAPS §0.4, M1-030).

Design positions live in the 2048 m design frame (x east, y north, origin SW). Every anchor also has
a real UTM position. If the real positions differ from the design ones by at most
``TRANSLATE_ONLY_M`` the design is just shifted; otherwise a translation plus a small rotation
(at most ``MAX_ROT_DEG``) is fitted by weighted least squares (weights 1/confidence²). The rotation
pivots on the frame centre so ``dx, dy`` stay readable. Real landmarks are never transformed.
"""
from __future__ import annotations

import math
from dataclasses import dataclass

TRANSLATE_ONLY_M = 150.0
MAX_ROT_DEG = 10.0
SNAP_MAX_M = 60.0
PIVOT = (1024.0, 1024.0)


class AnchorError(ValueError):
    pass


@dataclass(frozen=True)
class Anchor:
    id: str
    design: tuple[float, float]
    real: tuple[float, float]  # map-local metres (UTM minus the frame origin)
    confidence_m: float


def _check(anchors: list[Anchor]) -> None:
    if len(anchors) < 2:
        raise AnchorError("need at least 2 anchors to fit a transform")
    for a in anchors:
        if not a.confidence_m > 0:
            raise AnchorError(f"anchor {a.id}: confidence_m must be > 0")


def apply(points, sim: dict):
    """Applies ``sim`` (``dx, dy, rot_deg``) to design points; returns a list of (x, y)."""
    th = math.radians(sim["rot_deg"])
    c, s = math.cos(th), math.sin(th)
    px, py = PIVOT
    out = []
    for x, y in points:
        rx, ry = x - px, y - py
        out.append((px + c * rx - s * ry + sim["dx"], py + s * rx + c * ry + sim["dy"]))
    return out


def residuals(anchors: list[Anchor], sim: dict) -> list[float]:
    moved = apply([a.design for a in anchors], sim)
    return [math.hypot(m[0] - a.real[0], m[1] - a.real[1]) for m, a in zip(moved, anchors)]


def fit_similarity(anchors: list[Anchor]) -> dict:
    """Returns ``{dx, dy, rot_deg, applied, mode, rms_m, max_dev_m}`` (``applied`` stays False)."""
    _check(anchors)
    w = [1.0 / (a.confidence_m**2) for a in anchors]
    sw = sum(w)
    dev = [(a.real[0] - a.design[0], a.real[1] - a.design[1]) for a in anchors]
    max_dev = max(math.hypot(*d) for d in dev)
    if max_dev <= TRANSLATE_ONLY_M:
        sim = {
            "dx": sum(wi * d[0] for wi, d in zip(w, dev)) / sw,
            "dy": sum(wi * d[1] for wi, d in zip(w, dev)) / sw,
            "rot_deg": 0.0,
            "mode": "translate",
        }
    else:
        # Weighted 2-D Kabsch about the pivot: real - pivot ~ R (design - pivot) + t.
        px, py = PIVOT
        a_pts = [(a.design[0] - px, a.design[1] - py) for a in anchors]
        b_pts = [(a.real[0] - px, a.real[1] - py) for a in anchors]
        ca = (sum(wi * p[0] for wi, p in zip(w, a_pts)) / sw, sum(wi * p[1] for wi, p in zip(w, a_pts)) / sw)
        cb = (sum(wi * p[0] for wi, p in zip(w, b_pts)) / sw, sum(wi * p[1] for wi, p in zip(w, b_pts)) / sw)
        sxx = sxy = 0.0
        for wi, p, q in zip(w, a_pts, b_pts):
            ax, ay, bx, by = p[0] - ca[0], p[1] - ca[1], q[0] - cb[0], q[1] - cb[1]
            sxx += wi * (ax * bx + ay * by)
            sxy += wi * (ax * by - ay * bx)
        theta = math.atan2(sxy, sxx)
        rot = math.degrees(theta)
        if abs(rot) > MAX_ROT_DEG:
            raise AnchorError(
                f"fitted rotation {rot:.2f} deg exceeds the {MAX_ROT_DEG:.0f} deg limit (MAPS §0.4); "
                "re-check the anchors (H-051)"
            )
        c, s = math.cos(theta), math.sin(theta)
        sim = {
            "dx": cb[0] - (c * ca[0] - s * ca[1]),
            "dy": cb[1] - (s * ca[0] + c * ca[1]),
            "rot_deg": rot,
            "mode": "rotate",
        }
    sim["applied"] = False
    res = residuals(anchors, sim)
    sim["rms_m"] = math.sqrt(sum(r * r for r in res) / len(res))
    sim["max_dev_m"] = max_dev
    return sim


def apply_to_design(items: dict, sim: dict, preview: bool = False) -> list[dict]:
    """``items``: id -> (x, y). Keeps before and after for the audit trail. While
    ``sim['applied']`` is false the position is unchanged unless ``preview`` is set."""
    active = bool(sim.get("applied")) or preview
    ids = sorted(items)
    after = apply([items[i] for i in ids], sim) if active else [items[i] for i in ids]
    return [
        {"id": i, "before": list(items[i]), "after": [round(p[0], 3), round(p[1], 3)], "moved": active}
        for i, p in zip(ids, after)
    ]


def snap_to_standable(pos, standable, max_m: float = SNAP_MAX_M, step: float = 2.0) -> dict:
    """Nearest standable point within ``max_m`` (ring search, ``step`` metres). ``standable(x, y)``
    is false for water, solid buildings and slopes over 35 deg."""
    x, y = pos
    if standable(x, y):
        return {"pos": [x, y], "dist_m": 0.0, "note": None}
    best = None
    r = step
    while r <= max_m + 1e-9:
        n = max(8, int(2 * math.pi * r / step))
        for k in range(n):
            ang = 2 * math.pi * k / n
            cx, cy = x + r * math.cos(ang), y + r * math.sin(ang)
            if standable(cx, cy):
                best = (cx, cy, r)
                break
        if best:
            break
        r += step
    if best is None:
        raise AnchorError(f"no standable ground within {max_m:.0f} m of ({x:.1f}, {y:.1f})")
    return {
        "pos": [round(best[0], 3), round(best[1], 3)],
        "dist_m": round(best[2], 3),
        "note": f"snapped {best[2]:.0f} m from ({x:.1f}, {y:.1f}) to standable ground (MAPS §0.4.3)",
    }
