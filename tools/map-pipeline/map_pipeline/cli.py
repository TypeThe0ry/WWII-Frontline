"""CLI: `python -m map_pipeline m0 --seed 1 --out data/maps/m0` and `python -m map_pipeline validate DIR`."""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

from . import TOOL_VERSION
from .chunks import write_chunks
from .export import (
    DEFAULT_HQ,
    DEFAULT_PATROL,
    build_meta,
    choose_range,
    dump_json,
    manifest_for,
    pick_spawns,
    quantize,
    write_r16,
)
from .procedural import Terrain


def generate_m0(out: Path, seed: int, dem: str | None = None, window: tuple | None = None) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    if dem is None:
        terr = Terrain(seed)
        near, far = terr.near(), terr.far()
        generator = {"kind": "procedural", "seed": seed, "params": terr.params_json(), "tool_version": TOOL_VERSION}
        sources: list = []
        datum = "procedural"
    else:
        from .dem import load_dem  # lazy: needs GDAL

        if window is None:
            raise SystemExit("--dem requires --window E,N")
        near, far = load_dem(dem, *window)
        generator = {
            "kind": "dem",
            "seed": seed,
            "params": {"dem": Path(dem).name, "window_sw_e_n": list(window), "resample": "average"},
            "tool_version": TOOL_VERSION,
        }
        sources = []  # real sources are registered in M1-030/031
        datum = "dem"
    hmin, hmax = choose_range(near, far)
    qn, qf = quantize(near, hmin, hmax), quantize(far, hmin, hmax)
    write_r16(out / "heightmap_1m.r16", qn)
    write_r16(out / "heightmap_far_8m.r16", qf)
    rels = ["heightmap_1m.r16", "heightmap_far_8m.r16"] + write_chunks(out, qn, hmin, hmax)
    dump_json(out / "meta.json", build_meta(hmin=hmin, hmax=hmax, generator=generator, sources=sources, datum=datum))
    dump_json(out / "spawns.json", pick_spawns(DEFAULT_HQ, DEFAULT_PATROL, qn, hmin, hmax))
    dump_json(out / "manifest.json", manifest_for(out, rels))
    return {"hmin": hmin, "hmax": hmax, "files": len(rels)}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(prog="map_pipeline")
    sub = ap.add_subparsers(dest="cmd", required=True)
    m0 = sub.add_parser("m0", help="generate the M0 256 m terrain block")
    m0.add_argument("--seed", type=int, default=1)
    m0.add_argument("--out", default="data/maps/m0")
    m0.add_argument("--dem", help="GeoTIFF DEM (needs GDAL); default: procedural")
    m0.add_argument("--window", help="E,N of the window's south-west sample point (DEM CRS)")
    v = sub.add_parser("validate", help="validate a generated map directory")
    v.add_argument("dir")
    a = ap.parse_args(argv)

    if a.cmd == "m0":
        window = None
        if a.window:
            try:
                e, n = (float(t) for t in a.window.split(","))
            except ValueError:
                print("error: --window must be E,N", file=sys.stderr)
                return 2
            window = (e, n)
        t0 = time.perf_counter()
        try:
            info = generate_m0(Path(a.out), a.seed, a.dem, window)
        except Exception as e:  # GdalMissing, bad coverage ...
            print(f"error: {e}", file=sys.stderr)
            return 1
        print(f"m0 generated in {time.perf_counter() - t0:.2f}s: hmin={info['hmin']} hmax={info['hmax']} -> {a.out}")
        return 0

    from .validate import validate

    errs = validate(Path(a.dir))
    for e in errs:
        print(e)
    print("validate:", "FAILED" if errs else "ok")
    return 1 if errs else 0
