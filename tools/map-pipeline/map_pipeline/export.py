"""Quantisation, file writers, meta/spawns/manifest generation."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy as np

from . import TOOL_VERSION

NEAR_SAMPLES = 257
FAR_SAMPLES = 1025
DEQUANT = "h = hmin_m + u16 / 65535 * (hmax_m - hmin_m)"


def choose_range(*arrays: np.ndarray) -> tuple[float, float]:
    lo = min(float(a.min()) for a in arrays)
    hi = max(float(a.max()) for a in arrays)
    hmin = float(np.floor(lo))
    hmax = float(np.ceil(hi))
    if hmax <= hmin:
        hmax = hmin + 1.0
    return hmin, hmax


def quantize(h: np.ndarray, hmin: float, hmax: float) -> np.ndarray:
    q = np.rint((h - hmin) / (hmax - hmin) * 65535.0)
    return np.clip(q, 0, 65535).astype("<u2")


def dequantize(q: np.ndarray, hmin: float, hmax: float) -> np.ndarray:
    return hmin + q.astype(np.float64) / 65535.0 * (hmax - hmin)


def write_r16(path: Path, q: np.ndarray) -> None:
    path.write_bytes(np.ascontiguousarray(q.astype("<u2")).tobytes())


def read_r16(path: Path, samples: int) -> np.ndarray:
    raw = np.fromfile(path, dtype="<u2")
    if raw.size != samples * samples:
        raise ValueError(f"{path}: expected {samples * samples} samples, got {raw.size}")
    return raw.reshape(samples, samples)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dump_json(path: Path, obj) -> None:
    path.write_text(json.dumps(obj, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def slope_deg(h: np.ndarray, res_m: float = 1.0) -> np.ndarray:
    gz, gx = np.gradient(h, res_m)
    return np.degrees(np.arctan(np.hypot(gx, gz)))


def build_meta(*, hmin, hmax, generator: dict, sources: list, datum: str) -> dict:
    return {
        "schema": "frontline.map.meta/1",
        "map_id": "m0",
        "display_name": "M0 terrain block",
        "date_setting": "1944-06",
        "crs": {"epsg": 32630, "origin_e": 0, "origin_n": 0, "size_m": 256},
        "height": {"res_m": 1, "samples": NEAR_SAMPLES, "hmin_m": hmin, "hmax_m": hmax, "datum": datum},
        "far_ring": {"res_m": 8, "samples": FAR_SAMPLES, "extent_m": 8192},
        "dequant": DEQUANT,
        "layout": "row-major little-endian u16; row index = z (south to north), column = x (west to east); "
        "local metres, SW corner = (0,0); far ring is centred on the block (block SW corner at far sample 496,496)",
        "chunks": {"size_m": 64, "samples": 65, "count": 4},
        "generator": generator,
        "sources": sources,
        "_meta": {"status": "initial", "tunable": []},
    }


def pick_spawns(hq_pos: dict, patrol: list[list[int]], dq: np.ndarray, hmin: float, hmax: float) -> dict:
    h = dequantize(dq, hmin, hmax)

    def y_at(x, z):
        return round(float(h[int(z), int(x)]), 4)

    def hq(side):
        x, z = hq_pos[side]
        return {"pos": [x, z], "y_m": y_at(x, z), "radius_m": 40}

    return {
        "schema": "frontline.map.spawns/1",
        "hq": {"US": hq("US"), "GER": hq("GER")},
        "patrol_points": [
            {"id": f"p{i + 1}", "pos": [x, z], "y_m": y_at(x, z)} for i, (x, z) in enumerate(patrol)
        ],
        "kinds_supported": ["hq"],
        "_meta": {"status": "initial", "tunable": []},
    }


DEFAULT_HQ = {"US": [128, 56], "GER": [128, 200]}
DEFAULT_PATROL = [[40, 40], [216, 40], [64, 96], [192, 96], [64, 160], [192, 160], [40, 216], [216, 216]]


def manifest_for(out: Path, rel_paths: list[str]) -> dict:
    return {
        "schema": "frontline.map.manifest/1",
        "note": "SHA-256 of generated binaries; regenerate with the command in DEVELOPING.md",
        "tool_version": TOOL_VERSION,
        "files": {r: {"bytes": (out / r).stat().st_size, "sha256": sha256_file(out / r)} for r in sorted(rel_paths)},
    }
