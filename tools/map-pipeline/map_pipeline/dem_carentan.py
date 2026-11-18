"""Carentan DEM stage (M1-031): LiDAR tiles -> blended 1 m slice + 8 m far ring. Needs GDAL
(``osgeo``); everything numeric lives in ``dem_prep`` / ``slice_m1`` and is unit-tested without it."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from . import carentan, dem_prep, export, qa, slice_m1 as sl
from .dem import _warp, require_gdal, window_bounds

DST_SRS = "EPSG:32630"
NEAR_N, FAR_N, FAR_RES = sl.NEAR_N, sl.FAR_N, sl.FAR_RES
# Fixed extra check points inside the 1 m rectangle (local metres) so the check has >= 10 points.
CHECK_GRID = [(500, 600), (900, 900), (1300, 1300), (700, 1500), (1500, 700), (1100, 1600)]


def fill_far_outside_coverage(far: np.ndarray, nodata: np.ndarray) -> np.ndarray:
    """The 8 km ring is larger than the downloaded tiles: copy the nearest valid sample outward
    (4-neighbour dilation; far ring only, never the 1 m area)."""
    out = far.copy()
    todo = nodata.copy()
    while todo.any():
        valid = ~todo
        grown = np.zeros_like(todo)
        for ax, sh in ((0, 1), (0, -1), (1, 1), (1, -1)):
            v = np.roll(valid, sh, ax)
            val = np.roll(out, sh, ax)
            if ax == 0:
                v[0 if sh == 1 else -1, :] = False
            else:
                v[:, 0 if sh == 1 else -1] = False
            take = todo & v & ~grown
            out[take] = val[take]
            grown |= take
        if not grown.any():
            raise ValueError("far ring has no valid sample at all")
        todo &= ~grown
    return out


def build(incoming: Path, out: Path, root: Path) -> dict:
    gdal = require_gdal()
    files = dem_prep.find_inputs(incoming)
    vrt = gdal.BuildVRT("/vsimem/carentan_mnt.vrt", [str(f) for f in files])
    ox, oy = carentan.ORIGIN_E, carentan.ORIGIN_N
    near = _warp(gdal, vrt, window_bounds(ox, oy, 1.0, NEAR_N), 1.0, DST_SRS)
    near_mask = near <= -9998
    near, filled = dem_prep.fill_nodata(near, near_mask)  # raises above 1 %
    fe, fn = ox + sl.FAR_ORIGIN_M, oy + sl.FAR_ORIGIN_M
    far = _warp(gdal, vrt, window_bounds(fe, fn, FAR_RES, FAR_N), FAR_RES, DST_SRS)
    far_cov = float((far > -9998).mean())
    far = fill_far_outside_coverage(far, far <= -9998)
    # keep the ring continuous with the 1 m data at shared sample points
    lo = int((0 - sl.FAR_ORIGIN_M) // FAR_RES)  # far index at local x = 0
    far[lo : lo + 257, lo : lo + 257] = near[::8, ::8]
    offset = dem_prep.datum_offset(near)
    near_h, far_h = near - offset, far - offset
    heights, _ = sl.blend(near_h, far_h)
    hmin, hmax = dem_prep.round_range(float(min(heights.min(), far_h.min())), float(max(heights.max(), far_h.max())))
    out.mkdir(parents=True, exist_ok=True)
    export.write_r16(out / "heightmap_1m.r16", export.quantize(heights, hmin, hmax))
    export.write_r16(out / "heightmap_far_8m.r16", export.quantize(far_h, hmin, hmax))
    meta_path = out / "meta.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    meta["height"] = {
        "res_m": 1, "samples": NEAR_N, "hmin_m": hmin, "hmax_m": hmax, "datum": "IGN69",
        "offset_m": round(offset, 3), "offset_method": "p0.1 of the 1 m DEM (no water mask yet)",
        "source": "ign_lidar_hd", "row_order": "south_to_north",
        "nodata_filled_cells": filled, "far_ring_lidar_coverage": round(far_cov, 4),
        "inputs": dem_prep.input_manifest(files),
    }
    meta["far_ring"] = {"res_m": 8, "samples": FAR_N, "extent_m": 8192}
    meta["slice_m1"] = sl.meta_block()
    carentan.dump(meta_path, meta)
    # Self-consistency check at the anchors (NOT an independent DEM; see PI-080).
    pts = []
    spots = [(a["id"], a["e"] - ox, a["n"] - oy) for a in meta["anchors"]]
    spots += [(f"grid_{x}_{y}", float(x), float(y)) for x, y in CHECK_GRID]
    for sid, x, y in spots:
        if 0 <= x < 2048 and 0 <= y < 2048:
            pts.append({"id": sid, "e": ox + x, "n": oy + y,
                        "h_a": float(heights[int(round(y)), int(round(x))]),
                        "h_b": float(_point(gdal, vrt, ox + x, oy + y) - offset)})
    stats = qa.compare(pts) if len(pts) >= qa.MIN_POINTS else {"n": len(pts), "ok": None}
    qa.write_csv(root / "reports" / "m1-dem-check.csv", pts)
    return {"hmin": hmin, "hmax": hmax, "offset": offset, "filled": filled, "qa": stats,
            "near_sha": export.sha256_file(out / "heightmap_1m.r16"),
            "far_sha": export.sha256_file(out / "heightmap_far_8m.r16")}


def _point(gdal, vrt, e: float, n: float) -> float:
    """Raw 0.5 m LiDAR value at one point (nearest pixel) for the self-check."""
    arr = _warp(gdal, vrt, (e - 0.25, n - 0.25, e + 0.25, n + 0.25), 0.5, DST_SRS)
    return float(arr.mean())
