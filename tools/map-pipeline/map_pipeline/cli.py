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
    v.add_argument("--slice", default=None, help="m1: validate the Carentan slice data (M1-034)")
    v.add_argument("--repo", default=".")
    zb = sub.add_parser("zones-m1", help="generate M1 slice zones/spawns/routes/landmarks (M1-034)")
    zb.add_argument("--repo", default=".")
    zb.add_argument("--out", default="data/maps/carentan")
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
    dq = sub.add_parser("dem-qa", help="GDAL-free water-mask datum & canal-bed elevation check (M1-031)")
    dq.add_argument("dir", nargs="?", default="data/maps/carentan")
    dq.add_argument("--report", default="reports/m1-dem-water-check.json")
    om = sub.add_parser("osm", help="Carentan OSM Overpass extract -> vector layers (M1-032)")
    om.add_argument("--repo", default=".")
    om.add_argument("--out", default="data/maps/carentan")
    cr = sub.add_parser("check-roads", help="check vehicle_ok road connectivity (M1-032)")
    cr.add_argument("dir", nargs="?", default="data/maps/carentan")
    vv = sub.add_parser("validate-vectors", help="schema + range check of a map's vector layers")
    vv.add_argument("dir")
    ed = sub.add_parser("edits-1944", help="merge human 1944 inputs into the OSM draft (M1-033)")
    ed.add_argument("--repo", default=".")
    ed.add_argument("--out", default="data/maps/carentan")
    ed.add_argument("--incoming", default="assets/incoming/maps/carentan/traced")
    ch = sub.add_parser("chunks-m1", help="bake the M1 slice chunks (M1-035)")
    ch.add_argument("--repo", default=".")
    ch.add_argument("--out", default="data/maps/carentan")
    ch.add_argument("--dest", default="assets/processed/maps/carentan/m1/chunks")
    ch.add_argument("--workers", type=int, default=4)
    pc = sub.add_parser("pack-chunks", help="pack M1 chunks into FPAK v1 + manifest fragment (M1-035)")
    pc.add_argument("--repo", default=".")
    pc.add_argument("--chunks", default="assets/processed/maps/carentan/m1/chunks")
    pc.add_argument("--out", default="assets/processed/maps/carentan/m1/fpak")
    pc.add_argument("--manifest", default="assets/manifests/terrain_carentan.json")
    pc.add_argument("--report", default="reports/m1-chunk-bake.json")
    bi = sub.add_parser("building-interiors", help="bake shared M1 building graybox geometry")
    bi.add_argument("--out", default="data/maps/carentan")
    nv = sub.add_parser("nav", help="bake the M1 slice navmeshes (M1-036, needs tools/nav-bake deps)")
    nv.add_argument("--slice", default="m1")
    nv.add_argument("--repo", default=".")
    nv.add_argument("--out", default="data/maps/carentan")
    nv.add_argument("--tmp", default="assets/processed/maps/carentan/m1/nav")
    nq = sub.add_parser("nav-query", help="query a path on a baked FNAV navmesh (M1-036)")
    nq.add_argument("--from", dest="src", required=True, help="start x,z in metres")
    nq.add_argument("--to", dest="dst", required=True, help="goal x,z in metres")
    nq.add_argument("--mode", choices=("walk", "vehicle"), default="walk")
    nq.add_argument("--navmesh", default=None)
    nq.add_argument("--repo", default=".")
    a = ap.parse_args(argv)

    if a.cmd == "building-interiors":
        from .building_interiors import bake

        print(json.dumps(bake(Path(a.out))))
        return 0
    if a.cmd == "nav":
        import sys as _s
        _s.path.insert(0, str(Path(__file__).resolve().parent.parent))
        from nav import run as nav_run

        root = Path(a.repo)
        return nav_run.run(root, Path(a.out), root / "tools" / "nav-bake", root / a.tmp)
    if a.cmd == "nav-query":
        import sys as _s
        _s.path.insert(0, str(Path(__file__).resolve().parent.parent))
        from nav import run as nav_run

        root = Path(a.repo)
        navp = Path(a.navmesh) if a.navmesh else root / "data/maps/carentan/m1" / ("navmesh.bin" if a.mode == "walk" else "navmesh_vehicle.bin")
        return nav_run.query(navp, a.src, a.dst, root / "tools" / "nav-bake")
    if a.cmd == "chunks-m1":
        return _chunks(a)
    if a.cmd == "pack-chunks":
        return _pack_chunks(a)
    if a.cmd == "dem-qa":
        from . import qa

        rep = qa.water_elevation_check(Path(a.dir))
        if a.report:
            rp = Path(a.report)
            rp.parent.mkdir(parents=True, exist_ok=True)
            rp.write_text(json.dumps(rep, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(rep, indent=2, sort_keys=True))
        return 0
    if a.cmd == "check-roads":
        from . import osm

        roads = json.loads((Path(a.dir) / "roads.geojson").read_text(encoding="utf-8"))["features"]
        conn = osm.connectivity(roads)
        print(json.dumps(conn, indent=2, sort_keys=True))
        return 0 if conn["components"] >= 1 and conn["largest_m"] else 1
    if a.cmd == "zones-m1":
        from . import zones

        print(json.dumps(zones.build(Path(a.repo), Path(a.out)), ensure_ascii=False))
        return 0
    if a.cmd == "validate" and a.slice:
        from . import zones

        rep, ok = zones.validate_slice(Path(a.repo), Path(a.dir))
        rp = Path(a.repo) / "reports" / "m1-zones-validate.json"
        rp.write_text(json.dumps(rep, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        for k, v in rep["rules"].items():
            print(("PASS " if v["pass"] else "FAIL ") + k, v["detail"])
        return 0 if ok else 1
    if a.cmd == "edits-1944":
        from . import edits_1944

        return edits_1944.run(Path(a.repo), Path(a.out), Path(a.incoming))
    if a.cmd == "osm":
        return _osm(a)
    if a.cmd == "validate-vectors":
        return _validate_vectors(a)

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
    from .dem_carentan import build

    try:
        info = build(Path(a.incoming), Path(a.out), root)
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return dem_prep.EXIT_TOO_MUCH_NODATA if "no-data" in str(e) else 1
    print(json.dumps(info, indent=2))
    return 0


def _osm(a) -> int:
    from . import osm

    root = Path(a.repo)
    try:
        src = osm.find_input(root)
    except osm.OsmMissing as e:
        print(f"BLOCKED: {e}", file=sys.stderr)
        return osm.EXIT_MISSING_OSM
    meta_p = root / "data/maps/carentan/source/overpass_carentan.meta.json"
    ometa = json.loads(meta_p.read_text(encoding="utf-8")) if meta_p.is_file() else {}
    mp_path = root / "data/maps/carentan/meta.json"
    mp = json.loads(mp_path.read_text(encoding="utf-8"))
    crs = mp["crs"]
    origin = (crs["origin_e"], crs["origin_n"])
    raw_data = osm.load(src)
    layers, stats = osm.extract(raw_data, origin)
    out = Path(a.out)
    hashes = {k: osm.dump_layer(out / f"{k}.geojson", v) for k, v in layers.items()}
    osm.update_meta_sources(out / "meta.json", src, ometa)
    summ = osm.summary(layers, stats, hashes, ometa, raw_data=raw_data, origin=origin)
    rp = root / "reports" / "m1-osm-summary.json"
    rp.write_text(json.dumps(summ, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"counts": summ["counts"], "slice_m1": summ["slice_m1"], "connectivity": summ["road_connectivity"],
                      "intersection_utm_check": {k: summ["intersection_utm_check"][k] for k in ("sampled", "max_err_m", "ok")}}))
    return 0


def _validate_vectors(a) -> int:
    import jsonschema

    sd = Path(__file__).resolve().parent.parent / "schemas"
    bad = 0
    for k in ("roads", "water", "hedgerows", "buildings", "forests"):
        p = Path(a.dir) / f"{k}.geojson"
        if not p.is_file():
            print(f"missing {p}")
            bad += 1
            continue
        v = jsonschema.Draft202012Validator(json.loads((sd / f"{k}.schema.json").read_text(encoding="utf-8")))
        errs = list(v.iter_errors(json.loads(p.read_text(encoding="utf-8"))))
        for e in errs[:5]:
            print(f"{k}: {list(e.absolute_path)}: {e.message[:160]}")
        bad += len(errs)
        print(f"{k}: {len(errs)} errors")
    return 1 if bad else 0


def _chunks(a) -> int:
    import time

    from . import chunker

    t0 = time.time()
    out, dest, root = Path(a.out), Path(a.repo) / a.dest, Path(a.repo)
    ctx, res, entries = chunker.bake(out, dest, a.workers)
    rep = chunker.report(res, entries)
    rep["verify"] = chunker.verify(ctx, res, dest)
    rep["bake_seconds"] = round(time.time() - t0, 1)
    (out / "m1" / "chunks").mkdir(parents=True, exist_ok=True)
    (out / "m1" / "chunks" / "index.json").write_text(json.dumps(chunker.index_doc(entries), indent=1, sort_keys=True) + "\n", encoding="utf-8")
    (out / "m1" / "chunks" / "FORMAT.txt").write_text(chunker.FORMAT_DOC, encoding="utf-8")
    (root / "reports" / "m1-chunk-bake.json").write_text(json.dumps(rep, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({k: rep[k] for k in ("chunks", "bytes", "chunks_over_limit", "verify", "bake_seconds")}))
    return 1 if rep["chunks_over_limit"] else 0


def _pack_chunks(a) -> int:
    from . import pack_chunks as pc

    root = Path(a.repo)
    stats = pc.pack_chunks(root / a.chunks, root / a.out, root / a.manifest)
    rp = root / a.report
    if rp.is_file():
        rep = json.loads(rp.read_text(encoding="utf-8"))
        rep["fpak"] = stats
        rp.write_text(json.dumps(rep, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(stats, indent=2, sort_keys=True))
    return 0
