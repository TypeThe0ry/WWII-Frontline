"""CLI: `python -m map_pipeline m0 --seed 1 --out data/maps/m0` and `python -m map_pipeline validate DIR`."""
from __future__ import annotations

import argparse
import json
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
    cm = sub.add_parser("carentan-meta", help="write data/maps/carentan/meta.json + draft ledger rows (M1-030)")
    cm.add_argument("--repo", default=".")
    cm.add_argument("--out", default="data/maps/carentan")
    vs = sub.add_parser("validate-sources", help="check a map's data sources against LICENSES.md §3")
    vs.add_argument("dir")
    vs.add_argument("--repo", default=".")
    vs.add_argument("--report", default=None)
    dm = sub.add_parser("dem", help="Carentan DEM -> heightmaps (M1-031)")
    dm.add_argument("--incoming", default="assets/incoming/maps/carentan/dem")
    dm.add_argument("--out", default="data/maps/carentan")
    dm.add_argument("--repo", default=".")
    a = ap.parse_args(argv)

    if a.cmd == "dem":
        return _dem(a)

    if a.cmd == "carentan-meta":
        from . import carentan, licenses

        root = Path(a.repo)
        meta = carentan.build_meta(root)
        carentan.dump(Path(a.out) / "meta.json", meta)
        rows = root / "reports" / "m1-map-license-rows.md"
        rows.parent.mkdir(parents=True, exist_ok=True)
        rows.write_text(licenses.draft_rows(carentan.SOURCES), encoding="utf-8")
        print(f"wrote {Path(a.out) / 'meta.json'} and {rows}")
        return 0
    if a.cmd == "validate-sources":
        from . import carentan, licenses

        root = Path(a.repo)
        meta = json.loads((Path(a.dir) / "meta.json").read_text(encoding="utf-8"))
        errs = carentan.validate_meta(meta)
        report = licenses.check_sources(meta, licenses.parse_ledger(carentan._ledger(root)), root)
        if a.report:
            licenses.write_report(Path(a.report), report)
        for e in errs:
            print(e)
        for r in report:
            print(f"{r['status']:8} {r['id']}: {'; '.join(r['reasons']) or 'ledger OK + archive present'}")
        gaps = any(r["status"] != licenses.OK for r in report)
        # Gaps only fail the build once the human has marked the sources as checked (PI-077).
        enforce = bool(meta.get("m1_sources_checked"))
        bad = bool(errs) or (gaps and enforce)
        print("validate-sources:", "FAILED" if bad else ("ok" if not gaps else "ok (source gaps not enforced yet)"))
        return 1 if bad else 0

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


def _dem(a) -> int:
    """Exit codes: 3 = no DEM raster, 4 = a needed source is not OK in the ledger, 5 = too much
    no-data, 1 = other failure. The GDAL stage itself needs the ``osgeo`` bindings."""
    from . import carentan, dem_prep, licenses

    root = Path(a.repo)
    files = dem_prep.find_inputs(Path(a.incoming))
    if not files:
        print(
            f"error: no DEM raster (*.tif/*.vrt) in {a.incoming}.\n"
            "  missing: IGN LiDAR HD MNT (0.5 m) or RGE ALTI 1 m tiles covering the Carentan area\n"
            "  fix: download them into that folder (see tasks/M1-031 and PLAN_ISSUES PI-079)",
            file=sys.stderr,
        )
        return dem_prep.EXIT_MISSING_DEM
    meta = json.loads((Path(a.out) / "meta.json").read_text(encoding="utf-8"))
    ledger = licenses.parse_ledger(carentan._ledger(root))
    rep = {r["id"]: r for r in licenses.check_sources(meta, ledger, root)}
    if rep["ign_lidar_hd"]["status"] != licenses.OK and rep["ign_rge_alti"]["status"] != licenses.OK:
        print("error: neither ign_lidar_hd nor ign_rge_alti is OK in LICENSES.md §3; refusing to use the raster", file=sys.stderr)
        return dem_prep.EXIT_SOURCES_NOT_OK
    print(f"found {len(files)} raster(s); inputs: {dem_prep.input_manifest(files)}")
    try:
        from .dem import require_gdal

        require_gdal()
    except Exception as e:  # GdalMissing
        print(f"error: {e}", file=sys.stderr)
        return 1
    print("error: the GDAL mosaic/resample stage is wired in M1-031b once real tiles are on disk", file=sys.stderr)
    return 1
