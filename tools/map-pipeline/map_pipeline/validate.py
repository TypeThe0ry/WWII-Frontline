"""Validator for a generated map directory. Returns a list of error strings (empty = ok)."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from . import chunks as ck
from .export import dequantize, read_r16, sha256_file, slope_deg

MAX_SPAWN_SLOPE_DEG = 25.0
SIZE_M = 256


def _load(path: Path, errs: list):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        errs.append(f"{path.name}: cannot read/parse: {e}")
        return None


def validate(out: Path) -> list[str]:
    out = Path(out)
    errs: list[str] = []
    meta = _load(out / "meta.json", errs)
    spawns = _load(out / "spawns.json", errs)
    if meta is None:
        return errs
    hs = meta["height"]
    n, hmin, hmax = hs["samples"], hs["hmin_m"], hs["hmax_m"]
    if n != 257:
        errs.append(f"meta.height.samples must be 257, got {n}")
    if not hmin < hmax:
        errs.append("meta.height: hmin_m must be < hmax_m")
    fs = meta["far_ring"]["samples"]

    near = far = None
    try:
        near = read_r16(out / "heightmap_1m.r16", n)
    except (OSError, ValueError) as e:
        errs.append(f"heightmap_1m.r16: {e}")
    try:
        far = read_r16(out / "heightmap_far_8m.r16", fs)
    except (OSError, ValueError) as e:
        errs.append(f"heightmap_far_8m.r16: {e}")

    if near is not None:
        deq = dequantize(near, hmin, hmax)
        if deq.min() < hmin - 1e-9 or deq.max() > hmax + 1e-9:
            errs.append("heightmap_1m: dequantised sample outside [hmin, hmax]")
        if far is not None:
            c = (fs - 1) // 2 - 16
            if not np.array_equal(far[c : c + 33, c : c + 33], near[::8, ::8]):
                errs.append("far ring centre does not match near heightmap at 8 m sample points")
        errs += _check_chunks(out, near, hmin, hmax)
        if spawns is not None:
            errs += _check_spawns(spawns, deq)

    mf = out / "manifest.json"
    if mf.exists():
        man = _load(mf, errs) or {"files": {}}
        for rel, info in man["files"].items():
            p = out / rel
            if not p.exists():
                errs.append(f"manifest: {rel} missing")
            elif p.stat().st_size != info["bytes"] or sha256_file(p) != info["sha256"]:
                errs.append(f"manifest: {rel} size/sha256 mismatch")
    return errs


def _check_chunks(out: Path, near: np.ndarray, hmin: float, hmax: float) -> list[str]:
    errs: list[str] = []
    b: dict = {}
    for bz in range(ck.GRID):
        for bx in range(ck.GRID):
            bp, cp = out / "chunks" / f"b_{bx}_{bz}.bin", out / "chunks" / f"c_{bx}_{bz}.bin"
            try:
                raw = bp.read_bytes()
                if len(raw) != ck.B_BYTES:
                    errs.append(f"{bp.name}: expected {ck.B_BYTES} bytes, got {len(raw)}")
                    continue
                arr = np.frombuffer(raw, dtype="<u2").reshape(65, 65)
            except OSError as e:
                errs.append(f"{bp.name}: {e}")
                continue
            b[(bx, bz)] = arr
            if not np.array_equal(arr, ck.chunk_slice(near, bx, bz)):
                errs.append(f"{bp.name}: differs from heightmap slice")
            try:
                chmin, chmax, ch, mats = ck.decode_c(cp.read_bytes())
                if not np.array_equal(ch, arr):
                    errs.append(f"{cp.name}: heights differ from {bp.name}")
                if abs(chmin - hmin) > 1e-3 or abs(chmax - hmax) > 1e-3:
                    errs.append(f"{cp.name}: hmin/hmax differ from meta")
            except (OSError, ValueError) as e:
                errs.append(f"{cp.name}: {e}")
    for (bx, bz), arr in b.items():
        if (bx + 1, bz) in b and not np.array_equal(arr[:, 64], b[(bx + 1, bz)][:, 0]):
            errs.append(f"seam mismatch between chunks ({bx},{bz}) and ({bx + 1},{bz})")
        if (bx, bz + 1) in b and not np.array_equal(arr[64, :], b[(bx, bz + 1)][0, :]):
            errs.append(f"seam mismatch between chunks ({bx},{bz}) and ({bx},{bz + 1})")
    return errs


def _check_spawns(spawns: dict, deq: np.ndarray) -> list[str]:
    errs: list[str] = []
    slope = slope_deg(deq)
    pts = [(f"hq.{k}", v["pos"]) for k, v in spawns.get("hq", {}).items()]
    pts += [(p.get("id", "patrol"), p["pos"]) for p in spawns.get("patrol_points", [])]
    for name, (x, z) in pts:
        if not (0 <= x <= SIZE_M and 0 <= z <= SIZE_M):
            errs.append(f"spawns: {name} position {x},{z} outside 0..{SIZE_M}")
            continue
        s = float(slope[min(int(round(z)), 256), min(int(round(x)), 256)])
        if s > MAX_SPAWN_SLOPE_DEG:
            errs.append(f"spawns: {name} on slope {s:.1f} deg (> {MAX_SPAWN_SLOPE_DEG})")
    return errs
