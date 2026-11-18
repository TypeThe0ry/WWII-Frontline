"""Recast input triangle meshes for the M1 slice (M1-036): terrain at 1 m, walled enterable buildings,
solid other buildings, blocking hedge boxes, water holes. Output: little-endian float32/uint32 blobs
(x east, y up, z north) plus a JSON side file for the connectivity checks."""
from __future__ import annotations

import json
import math
import struct
from pathlib import Path

import numpy as np

from map_pipeline import buildings_annotate as ba, export, zones

RECT = ba.RECT
WALL_H, WALL_T, DOOR_GAP = 3.0, 0.3, 3.2
HEDGE_T, HEDGE_H = 0.6, 2.5


def _box(a, b, half, y0, y1):
    """Prism along segment a-b (xz plane) with half-width `half`; returns (verts, tris) outward-up winding."""
    dx, dz = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dz) or 1.0
    nx, nz = -dz / L * half, dx / L * half
    p = [(a[0] + nx, a[1] + nz), (b[0] + nx, b[1] + nz), (b[0] - nx, b[1] - nz), (a[0] - nx, a[1] - nz)]
    v = [(x, y0, z) for x, z in p] + [(x, y1, z) for x, z in p]
    t = [(4, 6, 5), (4, 7, 6), (0, 1, 2), (0, 2, 3), (0, 4, 5), (0, 5, 1), (1, 5, 6), (1, 6, 2), (2, 6, 7), (2, 7, 3), (3, 7, 4), (3, 4, 0)]
    return v, t


def terrain(h, hole, x0=RECT[0], z0=RECT[1], x1=RECT[2], z1=RECT[3]):
    """1 m grid triangles over [x0,x1]x[z0,z1]; cells whose centre is `hole` are skipped."""
    ix0, ix1, iz0, iz1 = int(x0), int(x1), int(z0), int(z1)
    nx, nz = ix1 - ix0 + 1, iz1 - iz0 + 1
    sub = h[iz0 : iz1 + 1, ix0 : ix1 + 1]
    xs, zs = np.meshgrid(np.arange(ix0, ix1 + 1, dtype=np.float32), np.arange(iz0, iz1 + 1, dtype=np.float32))
    pos = np.stack([xs, sub.astype(np.float32), zs], axis=-1).reshape(-1, 3)
    cc = np.arange((nz - 1) * (nx - 1))
    r, c = cc // (nx - 1), cc % (nx - 1)
    centre_blocked = hole[iz0 + r, ix0 + c]
    keep = ~centre_blocked
    r, c = r[keep], c[keep]
    i00 = r * nx + c
    i10, i01, i11 = i00 + 1, i00 + nx, i00 + nx + 1
    # (x,z)->(x+1,z+1) diagonal, up-facing winding for y-up with z north
    tri = np.concatenate([np.stack([i00, i01, i11], axis=-1), np.stack([i00, i11, i10], axis=-1)])
    return pos, tri.astype(np.uint32)


def build(out: Path, vehicle: bool):
    meta = json.loads((out / "meta.json").read_text(encoding="utf-8"))
    hm = meta["height"]
    q = export.read_r16(out / "heightmap_1m.r16", hm["samples"])
    h = export.dequantize(q, hm["hmin_m"], hm["hmax_m"])
    L = {k: json.loads((out / f"{k}.geojson").read_text(encoding="utf-8"))["features"] for k in ("buildings", "water", "hedgerows")}
    bs = [f for f in L["buildings"] if ba.in_rect(*ba.centroid(f["geometry"]["coordinates"][0]), rect=(RECT[0] - 40, RECT[1] - 40, RECT[2] + 40, RECT[3] + 40))]
    solid = [f for f in bs if vehicle or not f["properties"]["enterable"]]
    grid_all = zones.build_grid([], L["water"], res=1.0)
    hole = grid_all.b.copy()
    sg = zones.build_grid(solid, [], res=1.0)
    hole |= sg.b
    pos, tri = terrain(h, hole)
    verts, tris = [tuple(p) for p in pos.tolist()], [tuple(t) for t in tri.tolist()]
    nwalls = 0
    if not vehicle:
        for f in bs:
            if not f["properties"]["enterable"]:
                continue
            ring = [tuple(p) for p in f["geometry"]["coordinates"][0]]
            door = (f["properties"].get("door") or {}).get("center")
            for a, b in zip(ring, ring[1:]):
                segs = [(a, b)]
                if door and _dist_pt_seg(door, a, b) < DOOR_GAP:
                    segs = _cut(a, b, door, DOOR_GAP / 2)
                for s0, s1 in segs:
                    mid = ((s0[0] + s1[0]) / 2, (s0[1] + s1[1]) / 2)
                    y0 = float(h[int(mid[1]), int(mid[0])]) - 0.4
                    v, t = _box(s0, s1, WALL_T / 2, y0, y0 + WALL_H + 0.4)
                    n = len(verts)
                    verts += v
                    tris += [(n + i, n + j, n + k) for i, j, k in t]
                    nwalls += 1
    nh = 0
    for f in L["hedgerows"]:
        if not f["properties"]["vehicle_blocking"]:
            continue
        pts = f["geometry"]["coordinates"]
        for a, b in zip(pts, pts[1:]):
            mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
            if not (RECT[0] <= mid[0] <= RECT[2] and RECT[1] <= mid[1] <= RECT[3]):
                continue
            y0 = float(h[int(mid[1]), int(mid[0])]) - 0.4
            v, t = _box(a, b, HEDGE_T / 2, y0, y0 + HEDGE_H)
            n = len(verts)
            verts += v
            tris += [(n + i, n + j, n + k) for i, j, k in t]
            nh += 1
    return np.array(verts, dtype=np.float32), np.array(tris, dtype=np.uint32), {"walls": nwalls, "hedge_boxes": nh, "tris": len(tris)}, h


def _dist_pt_seg(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = dx * dx + dy * dy
    t = 0.0 if L == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def _cut(a, b, door, half):
    """Remove [-half, +half] around the door's projection on a-b; returns remaining pieces."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dy)
    t = ((door[0] - a[0]) * dx + (door[1] - a[1]) * dy) / (L * L)
    d = half / L
    out = []
    if t - d > 0:
        out.append((a, (a[0] + dx * (t - d), a[1] + dy * (t - d))))
    if t + d < 1:
        out.append(((a[0] + dx * (t + d), a[1] + dy * (t + d)), b))
    return out


def write(path: Path, verts, tris):
    with path.open("wb") as fh:
        fh.write(struct.pack("<II", len(verts), len(tris)))
        fh.write(verts.astype("<f4").tobytes())
        fh.write(tris.astype("<u4").tobytes())
