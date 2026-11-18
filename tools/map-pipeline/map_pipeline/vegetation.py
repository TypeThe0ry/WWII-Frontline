"""Vegetation placement for a chunk: forests, hedge-row trees, shrubs, grass seed (M1-035)."""
from __future__ import annotations

import hashlib
import random

import numpy as np

from . import apply_edits, splat

MAX_GRASS, MAX_SHRUBS, MAX_TREES = 4096, 256, 192
SPECIES = {"oak": 1, "poplar": 2, "orchard_apple": 3, "conifer": 4, "shrub_hawthorn": 10, "shrub_bramble": 11}
TREE_SPACING_M = 6.0
HEDGE_TREE_SPACING_M = 9.0
HEDGE_SHRUB_SPACING_M = 4.0


def seed_for(map_id: str, bx: int, bz: int) -> int:
    return int.from_bytes(hashlib.sha256(f"{map_id}/{bx}/{bz}".encode()).digest()[:8], "little")


def _q(v):  # chunk-local metres -> u16
    return int(round(min(max(v, 0.0), 64.0) / 64.0 * 65535))


def _allowed(m, bx, bz, x, y):
    """True if (x, y) is free of road, building, water, marsh (window masks)."""
    c = int(x - (bx * 64 - splat.HALO))
    r = int(y - (bz * 64 - splat.HALO))
    if not (0 <= r < m["road"].shape[0] and 0 <= c < m["road"].shape[1]):
        return False
    return not (m["road"][r, c] or m["dirt_road"][r, c] or m["building_buf"][r, c] or m["water"][r, c] or m["marsh"][r, c])


def place(map_id, bx, bz, m, forests, hedges, density_of):
    rng = random.Random(seed_for(map_id, bx, bz))
    x0, z0 = bx * 64.0, bz * 64.0
    trees, shrubs = [], []
    # forests: jittered grid inside forest mask
    for gy in np.arange(0.0, 64.0, TREE_SPACING_M):
        for gx in np.arange(0.0, 64.0, TREE_SPACING_M):
            x = x0 + gx + rng.uniform(0, TREE_SPACING_M)
            y = z0 + gy + rng.uniform(0, TREE_SPACING_M)
            if not (x0 <= x < x0 + 64 and z0 <= y < z0 + 64):
                continue
            c, r = int(x - (x0 - splat.HALO)), int(y - (z0 - splat.HALO))
            if m["forest"][r, c] and _allowed(m, bx, bz, x, y) and rng.random() < density_of:
                trees.append((x - x0, y - z0, "conifer" if rng.random() < 0.2 else "oak"))
    # hedge-row oaks and shrubs along hedge polylines
    for h in hedges:
        pts = h["pts"]
        acc_t = acc_s = 0.0
        for a, b in zip(pts, pts[1:]):
            L = ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5
            n = max(1, int(L))
            for i in range(n):
                t = i / n
                x, y = a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])
                acc_t += L / n
                acc_s += L / n
                if not (x0 <= x < x0 + 64 and z0 <= y < z0 + 64):
                    continue
                if acc_t >= HEDGE_TREE_SPACING_M:
                    acc_t = 0.0
                    if _allowed(m, bx, bz, x, y):
                        trees.append((x - x0, y - z0, "oak"))
                if acc_s >= HEDGE_SHRUB_SPACING_M:
                    acc_s = 0.0
                    if _allowed(m, bx, bz, x, y):
                        shrubs.append((x - x0, y - z0, "shrub_hawthorn"))
    trees, shrubs = trees[:MAX_TREES], shrubs[:MAX_SHRUBS]
    def enc(items, lo, hi):
        out = []
        for x, y, sp in items:
            out.append((_q(x), _q(y), SPECIES[sp], rng.randint(lo, hi), rng.randint(0, 255)))
        return out
    return enc(trees, 180, 255), enc(shrubs, 150, 230), rng.getrandbits(32)
