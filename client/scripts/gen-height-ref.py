#!/usr/bin/env python3
"""Golden height samples for client/src/render/terrain/terrain-data.test.ts (M0-023 acceptance 4).

Independent numpy (float64) implementation of the documented sampling: bilinear over the 1 m map inside the
256 m block, and outside it the 8 m far map blended toward the clamped near-edge value over 16 m with a
smoothstep. Run after (re)generating data/maps/m0:

    python3 client/scripts/gen-height-ref.py
"""
import hashlib
import json
import pathlib

import numpy as np

root = pathlib.Path(__file__).resolve().parents[2]
mdir = root / "data" / "maps" / "m0"
meta = json.loads((mdir / "meta.json").read_text())
hmin, hmax = meta["height"]["hmin_m"], meta["height"]["hmax_m"]
ns, fs = meta["height"]["samples"], meta["far_ring"]["samples"]
size, extent, fres = meta["crs"]["size_m"], meta["far_ring"]["extent_m"], meta["far_ring"]["res_m"]
raw_n = (mdir / "heightmap_1m.r16").read_bytes()
raw_f = (mdir / "heightmap_far_8m.r16").read_bytes()
near = hmin + np.frombuffer(raw_n, dtype="<u2").reshape(ns, ns).astype(np.float64) / 65535.0 * (hmax - hmin)
far = hmin + np.frombuffer(raw_f, dtype="<u2").reshape(fs, fs).astype(np.float64) / 65535.0 * (hmax - hmin)
far_origin = size / 2 - extent / 2


def bilinear(grid, n, res, ox, x, z):
    u = min(max((x - ox) / res, 0.0), n - 1.0)
    v = min(max((z - ox) / res, 0.0), n - 1.0)
    i0, j0 = min(int(np.floor(u)), n - 2), min(int(np.floor(v)), n - 2)
    fx, fz = u - i0, v - j0
    a, b = grid[j0, i0], grid[j0, i0 + 1]
    c, d = grid[j0 + 1, i0], grid[j0 + 1, i0 + 1]
    return (a * (1 - fx) + b * fx) * (1 - fz) + (c * (1 - fx) + d * fx) * fz


def surface(x, z):
    d = max(-x, x - size, -z, z - size, 0.0)
    nh = bilinear(near, ns, 1.0, 0.0, x, z)
    if d <= 0:
        return nh
    t = min(d / 16.0, 1.0)
    w = 1.0 - t * t * (3.0 - 2.0 * t)
    fh = bilinear(far, fs, fres, far_origin, x, z)
    return fh + (nh - fh) * w


rng = np.random.default_rng(20240523)
inside = [[float(x), float(z)] for x, z in rng.uniform(0.0, size, size=(1000, 2))]
outside = []
while len(outside) < 1000:
    x, z = rng.uniform(far_origin + 8, far_origin + extent - 8, size=2)
    if not (0 <= x <= size and 0 <= z <= size):
        outside.append([float(x), float(z)])
# a handful of exact corner/edge points
edge = [[0.0, 0.0], [size, 0.0], [0.0, size], [size, size], [128.0, 0.0], [0.0, 128.0], [255.5, 255.5], [-8.0, 100.0], [264.0, 100.0], [100.0, -16.0], [100.0, 272.0]]
out = {
    "map": "m0",
    "r16_sha256": {"heightmap_1m.r16": hashlib.sha256(raw_n).hexdigest(), "heightmap_far_8m.r16": hashlib.sha256(raw_f).hexdigest()},
    "inside": [[x, z, round(surface(x, z), 7)] for x, z in inside],
    "outside": [[x, z, round(surface(x, z), 7)] for x, z in outside],
    "edge": [[x, z, round(surface(x, z), 7)] for x, z in edge],
}
dst = root / "client" / "src" / "render" / "terrain" / "fixtures" / "height_ref_m0.json"
dst.write_text(json.dumps(out, separators=(",", ":")) + "\n")
print(f"wrote {dst} ({dst.stat().st_size} bytes)")
