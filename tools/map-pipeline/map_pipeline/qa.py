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
