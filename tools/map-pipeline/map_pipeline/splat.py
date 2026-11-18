"""Splat rules engine: rasterised GIS layers -> 8-layer weights (M1-035). 1 m pixels, deterministic."""
from __future__ import annotations

import math

import numpy as np

from . import apply_edits

LAYERS = ("grass", "dirt", "mud", "road", "cobble", "rock", "sand", "forest_floor")
L = {n: i for i, n in enumerate(LAYERS)}
HALO = 4  # px around the chunk so blurs and buffers are seamless
SLOPE_ROCK_DEG = 35.0
WATER_EDGE_M = 3.0
BUILDING_BUFFER_M = 2.0


def window(bx, bz, size=64, halo=HALO):
    x0, y0 = bx * size - halo, bz * size - halo
    n = size + 2 * halo
    xs = x0 + np.arange(n) + 0.5
    ys = y0 + np.arange(n) + 0.5
    X, Y = np.meshgrid(xs, ys)
    return X, Y


def dist_polyline(X, Y, pts):
    d = np.full(X.shape, np.inf)
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        Ls = dx * dx + dy * dy
        t = np.zeros_like(X) if Ls == 0 else np.clip(((X - a[0]) * dx + (Y - a[1]) * dy) / Ls, 0, 1)
        d = np.minimum(d, np.hypot(X - a[0] - t * dx, Y - a[1] - t * dy))
    return d


def _near(pts, X, Y, pad):
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return not (max(xs) + pad < X.min() or min(xs) - pad > X.max() or max(ys) + pad < Y.min() or min(ys) - pad > Y.max())


def masks(X, Y, layers):
    """Rasterise the vector layers over the window -> dict of boolean/float arrays."""
    m = {k: np.zeros(X.shape, dtype=bool) for k in ("building", "building_buf", "water", "water_edge_sand", "water_edge_mud",
                                                      "forest", "road", "dirt_road", "marsh")}
    for f in layers["buildings"]:
        ring = f["geometry"]["coordinates"][0]
        if not _near(ring, X, Y, BUILDING_BUFFER_M):
            continue
        r = [tuple(p) for p in ring[:-1]]
        ins = apply_edits._inside(X, Y, r)
        m["building"] |= ins
        m["building_buf"] |= ins | (apply_edits._dist(X, Y, r) <= BUILDING_BUFFER_M)
    for f in layers["water"]:
        p, g = f["properties"], f["geometry"]
        if g["type"] == "LineString":
            if not _near(g["coordinates"], X, Y, 20):
                continue
            half = p.get("width_m", 2.0) / 2.0
            d = dist_polyline(X, Y, g["coordinates"])
            m["water"] |= d <= half
            if p["kind"] in ("river", "canal"):
                m["water_edge_sand"] |= (d > half) & (d <= half + WATER_EDGE_M)
            else:
                m["water_edge_mud"] |= (d > half) & (d <= half + WATER_EDGE_M)
        else:
            ring = g["coordinates"][0]
            if not _near(ring, X, Y, 4):
                continue
            r = [tuple(q) for q in ring[:-1]]
            ins = apply_edits._inside(X, Y, r)
            if p["kind"] == "flood_marsh":
                m["marsh"] |= ins
                continue
            wet = p["kind"] == "wetland"
            m["water"] |= ins & (not wet)
            m["marsh"] |= ins & wet
            edge = ~ins & (apply_edits._dist(X, Y, r) <= WATER_EDGE_M)
            m["water_edge_sand" if not wet else "water_edge_mud"] |= edge
    for f in layers["forests"]:
        ring = f["geometry"]["coordinates"][0]
        if _near(ring, X, Y, 0):
            m["forest"] |= apply_edits._inside(X, Y, [tuple(q) for q in ring[:-1]])
    for f in layers["roads"]:
        p, pts = f["properties"], f["geometry"]["coordinates"]
        if not _near(pts, X, Y, p["width_m"]):
            continue
        d = dist_polyline(X, Y, pts) <= p["width_m"] / 2.0
        if p["class"] in ("paved", "causeway") and not p["bridge"]:
            m["road"] |= d
        elif p["class"] in ("dirt", "path", "rail") and not p["bridge"]:
            m["dirt_road"] |= d
    return m


def slope_deg(hwin, res=1.0):
    gy, gx = np.gradient(hwin, res)
    return np.degrees(np.arctan(np.hypot(gx, gy)))


def classify(m, slope, urban: bool):
    """Priority painter -> int layer index per pixel (low priority first)."""
    out = np.full(slope.shape, L["grass"], dtype=np.uint8)
    out[m["forest"]] = L["forest_floor"]
    out[m["water_edge_sand"]] = L["sand"]
    out[m["water_edge_mud"]] = L["mud"]
    out[m["marsh"]] = L["mud"]
    out[m["water"]] = L["mud"]
    out[m["building_buf"]] = L["dirt"]
    out[m["dirt_road"]] = L["dirt"]
    out[m["road"]] = L["cobble"] if urban else L["road"]
    out[slope > SLOPE_ROCK_DEG] = L["rock"]
    return out


def blur_weights(idx, sigma_px=1.5):
    """One-hot -> separable gaussian (radius 3) -> float weights (H, W, 8)."""
    k = np.exp(-0.5 * (np.arange(-3, 4) / sigma_px) ** 2)
    k /= k.sum()
    w = np.stack([(idx == i).astype(np.float64) for i in range(len(LAYERS))], axis=-1)
    for ax in (0, 1):
        pad = [(0, 0)] * 3
        pad[ax] = (3, 3)
        p = np.pad(w, pad, mode="edge")
        w = sum(k[i] * np.take(p, np.arange(i, i + w.shape[ax]), axis=ax) for i in range(7))
    return w


def quantize255(w):
    """Largest-remainder rounding so every pixel sums to exactly 255."""
    w = w / w.sum(axis=-1, keepdims=True)
    raw = w * 255.0
    q = np.floor(raw).astype(np.int32)
    rem = 255 - q.sum(axis=-1)
    frac = raw - q
    order = np.argsort(-frac, axis=-1, kind="stable")
    ranks = np.argsort(order, axis=-1, kind="stable")
    q += (ranks < rem[..., None]).astype(np.int32)
    return q.astype(np.uint8)


def splat_chunk(bx, bz, layers, hwin_full, size=64):
    """hwin_full: heights (m) over the same HALO window. Returns (uint8[64,64,8], masks, idx)."""
    X, Y = window(bx, bz, size)
    m = masks(X, Y, layers)
    urban = int(m["building"].sum()) > 0.04 * m["building"].size
    idx = classify(m, slope_deg(hwin_full), urban)
    w = blur_weights(idx)[HALO : HALO + size, HALO : HALO + size]
    return quantize255(w), m, idx
