"""Terrain edits baked into the 1 m height grid (M1-033). Pure numpy, deterministic."""
from __future__ import annotations

import math

import numpy as np

MODES = ("set_height", "raise", "lower", "flatten", "embankment")
WARN_DELTA_M = 3.0
MAX_DELTA_M = 5.0


def smoothstep(t):
    t = np.clip(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def _inside(xs, ys, ring):
    """Even-odd test, ring closed or open; xs/ys broadcastable arrays."""
    inside = np.zeros(np.broadcast(xs, ys).shape, dtype=bool)
    n = len(ring)
    for k in range(n):
        x0, y0 = ring[k]
        x1, y1 = ring[(k + 1) % n]
        if y0 == y1:
            continue
        cond = (y0 > ys) != (y1 > ys)
        xi = (x1 - x0) * (ys - y0) / (y1 - y0) + x0
        inside ^= cond & (xs < xi)
    return inside


def _dist(xs, ys, ring):
    d = np.full(np.broadcast(xs, ys).shape, np.inf)
    n = len(ring)
    for k in range(n):
        (x0, y0), (x1, y1) = ring[k], ring[(k + 1) % n]
        dx, dy = x1 - x0, y1 - y0
        L = dx * dx + dy * dy
        t = np.zeros_like(d) if L == 0 else np.clip(((xs - x0) * dx + (ys - y0) * dy) / L, 0, 1)
        d = np.minimum(d, np.hypot(xs - x0 - t * dx, ys - y0 - t * dy))
    return d


def weights(shape, ring, transition_m, res_m=1.0):
    """(weight array, row/col slice): 1 inside, smoothstep falloff over transition_m outside."""
    xs_ = [p[0] for p in ring]
    ys_ = [p[1] for p in ring]
    pad = transition_m + res_m
    c0 = max(0, int(math.floor((min(xs_) - pad) / res_m)))
    c1 = min(shape[1] - 1, int(math.ceil((max(xs_) + pad) / res_m)))
    r0 = max(0, int(math.floor((min(ys_) - pad) / res_m)))
    r1 = min(shape[0] - 1, int(math.ceil((max(ys_) + pad) / res_m)))
    if c1 < c0 or r1 < r0:
        return None, None
    X, Y = np.meshgrid(np.arange(c0, c1 + 1) * res_m, np.arange(r0, r1 + 1) * res_m)
    ins = _inside(X, Y, ring)
    w = ins.astype(np.float64)
    if transition_m > 0:
        d = _dist(X, Y, ring)
        w = np.where(ins, 1.0, smoothstep(1.0 - d / transition_m))
    return w, (slice(r0, r1 + 1), slice(c0, c1 + 1))


def apply_terrain_edits(h, edits, res_m: float = 1.0):
    """h: 2D float array (row = y south->north, col = x). edits: GeoJSON features.
    Returns (new_h, report). Input is never modified; same input -> same output."""
    out = np.array(h, dtype=np.float64, copy=True)
    feats = sorted(enumerate(edits), key=lambda t: (t[1]["properties"].get("order", 0), t[0]))
    per, warns = [], []
    for _, f in feats:
        p = f["properties"]
        mode = p["mode"]
        if mode not in MODES:
            raise ValueError(f"unknown edit mode {mode!r} in {p.get('id')}")
        ring = [tuple(c) for c in f["geometry"]["coordinates"][0]]
        if ring[0] == ring[-1]:
            ring = ring[:-1]
        w, sl = weights(out.shape, ring, float(p.get("transition_m", 0.0)), res_m)
        if w is not None:
            cur = out[sl].copy()
            X, Y = np.meshgrid(np.arange(sl[1].start, sl[1].stop) * res_m, np.arange(sl[0].start, sl[0].stop) * res_m)
            ins = _inside(X, Y, ring)
            if mode == "set_height":
                tgt = float(p["target_h_m"])
            elif mode == "raise":
                tgt = cur + float(p["delta_m"])
            elif mode == "lower":
                tgt = cur - abs(float(p["delta_m"]))
            elif mode == "flatten":
                tgt = float(cur[ins].mean()) if ins.any() else cur
            else:  # embankment: raise to target but never dig
                tgt = np.maximum(cur, float(p["target_h_m"]))
            updated = cur + (np.asarray(tgt) - cur) * w
            out[sl] = updated
            d_sub = updated - cur
            ch = d_sub != 0
            if ch.any():
                mx = float(np.abs(d_sub).max())
                mean_abs = float(np.abs(d_sub[ch]).mean())
                r_off, c_off = sl[0].start, sl[1].start
                rows, cols = np.nonzero(ch)
                bbox = [int(cols.min() + c_off), int(rows.min() + r_off), int(cols.max() + c_off), int(rows.max() + r_off)]
                cells_changed = int(ch.sum())
            else:
                mx = mean_abs = 0.0
                bbox = None
                cells_changed = 0
        else:
            mx = mean_abs = 0.0
            bbox = None
            cells_changed = 0
        per.append({"id": p.get("id"), "mode": mode, "cells_changed": cells_changed,
                    "max_abs_dh_m": round(mx, 4), "mean_abs_dh_m": round(mean_abs, 4),
                    "bbox_cells": bbox})
        if mx > WARN_DELTA_M and not p.get("reason"):
            warns.append(f"{p.get('id')}: |dh| {mx:.2f} m > {WARN_DELTA_M} m without reason")
    tot = out - np.asarray(h, dtype=np.float64)
    allch = tot != 0
    report = {"cells_changed": int(allch.sum()), "max_abs_dh_m": round(float(np.abs(tot).max()), 4) if allch.any() else 0.0,
              "polygons": per, "warnings": warns, "exceeds_max_delta": bool(allch.any() and np.abs(tot).max() > MAX_DELTA_M)}
    return out, report
