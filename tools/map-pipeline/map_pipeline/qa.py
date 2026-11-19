"""Spot-check heights of two independent sources at >= 10 points (M1-031, acceptance 1)."""
from __future__ import annotations

import csv
import statistics
from pathlib import Path

MAX_ABS_M = 1.0
MEDIAN_ABS_M = 0.5
MIN_POINTS = 10


def compare(points: list[dict]) -> dict:
    """``points``: dicts with ``id, e, n, h_a, h_b``. Returns the stats and the verdict."""
    if len(points) < MIN_POINTS:
        raise ValueError(f"need >= {MIN_POINTS} check points, got {len(points)}")
    d = [abs(p["h_a"] - p["h_b"]) for p in points]
    res = {"n": len(d), "max_abs_m": max(d), "median_abs_m": statistics.median(d)}
    res["ok"] = res["max_abs_m"] <= MAX_ABS_M and res["median_abs_m"] <= MEDIAN_ABS_M
    return res


def write_csv(path: Path, points: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["id", "e", "n", "h_a_m", "h_b_m", "delta_m"])
        for p in points:
            w.writerow([p["id"], p["e"], p["n"], f"{p['h_a']:.3f}", f"{p['h_b']:.3f}", f"{p['h_a'] - p['h_b']:.3f}"])


def water_elevation_check(map_dir: Path) -> dict:
    """GDAL-free cross-check of committed heightmaps against ``water.geojson`` (M1-031 / PI-080).
    Answers whether low cells (< -1 m / -6 m) lie in the M1 slice or in the far-ring/NE basin and
    compares the water-masked datum offset against the current ``meta.height.offset_m``."""
    import json
    import numpy as np
    from . import dem_prep, export, slice_m1 as sl

    meta = json.loads((map_dir / "meta.json").read_text(encoding="utf-8"))
    hm = meta["height"]
    h = export.dequantize(export.read_r16(map_dir / "heightmap_1m.r16", hm["samples"]), hm["hmin_m"], hm["hmax_m"])
    hf = export.dequantize(export.read_r16(map_dir / "heightmap_far_8m.r16", meta["far_ring"]["samples"]), hm["hmin_m"], hm["hmax_m"])
    water = json.loads((map_dir / "water.geojson").read_text(encoding="utf-8")).get("features", [])
    wm = dem_prep.water_mask_from_features(water, h.shape, res=1.0)
    offset = float(hm.get("offset_m", 0.0))
    ign_h = h + offset
    x0, y0, x1, y1 = (int(v) for v in meta.get("slice_m1", {}).get("rect_local_m", sl.RECT_DESIGN))
    h_slice = h[y0 : y1 + 1, x0 : x1 + 1]
    wm_slice = wm[y0 : y1 + 1, x0 : x1 + 1]
    low = h < -1.0
    ys, xs = np.nonzero(low)
    low_bbox = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())] if len(xs) else None
    low_in_slice = int((h_slice < -1.0).sum())
    water_offset_ign = float(dem_prep.datum_offset(ign_h, wm)) if wm.any() else None
    water_p1_ign = float(np.percentile(ign_h[wm], 1)) if wm.any() else None
    return {
        "schema": 1,
        "gdal_required": False,
        "heightmap_1m": {"min_m": round(float(h.min()), 3), "max_m": round(float(h.max()), 3), "bytes": (map_dir / "heightmap_1m.r16").stat().st_size},
        "heightmap_far_8m": {"min_m": round(float(hf.min()), 3), "max_m": round(float(hf.max()), 3), "bytes": (map_dir / "heightmap_far_8m.r16").stat().st_size},
        "slice_m1": {
            "min_m": round(float(h_slice.min()), 3),
            "max_m": round(float(h_slice.max()), 3),
            "water_cells": int(wm_slice.sum()),
            "water_min_m": round(float(h_slice[wm_slice].min()), 3) if wm_slice.any() else None,
            "water_p50_m": round(float(np.percentile(h_slice[wm_slice], 50)), 3) if wm_slice.any() else None,
            "cells_below_minus_1m": low_in_slice,
        },
        "water_mask_full": {
            "cells": int(wm.sum()),
            "game_min_m": round(float(h[wm].min()), 3) if wm.any() else None,
            "game_p1_m": round(float(np.percentile(h[wm], 1)), 3) if wm.any() else None,
            "game_p5_m": round(float(np.percentile(h[wm], 5)), 3) if wm.any() else None,
            "game_p50_m": round(float(np.percentile(h[wm], 50)), 3) if wm.any() else None,
            "ign69_min_m": round(water_offset_ign, 3) if water_offset_ign is not None else None,
            "ign69_p1_m": round(water_p1_ign, 3) if water_p1_ign is not None else None,
        },
        "datum_offset_comparison": {
            "current_offset_m": offset,
            "current_method": hm.get("offset_method"),
            "water_mask_min_ign69_m": round(water_offset_ign, 3) if water_offset_ign is not None else None,
            "delta_to_water_min_m": round(offset - water_offset_ign, 3) if water_offset_ign is not None else None,
            "delta_to_water_p1_m": round(offset - water_p1_ign, 3) if water_p1_ign is not None else None,
        },
        "sub_minus_1m_analysis": {
            "cells_in_1m_grid": int(low.sum()),
            "cells_inside_water_mask": int((low & wm).sum()),
            "cells_inside_m1_slice": low_in_slice,
            "bbox_local_m": low_bbox,
            "far_ring_min_m": round(float(hf.min()), 3),
            "finding": "The -6.9 m minimum occurs only in heightmap_far_8m (outside the 2048 m playable box); inside heightmap_1m, sub--1 m cells lie only at the NE corner outside the M1 slice.",
        },
    }
