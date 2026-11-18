"""M1-033 driver: merge human inputs (traced hedgerows, flood, removal list, terrain edits) into the
OSM draft layers. Missing human inputs => status partial, exit code 4 (nothing is invented)."""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from . import apply_edits, clean, export, hedgerow_tools as ht, modern_filter, osm

EXIT_PARTIAL = 4
RECT = (380.0, 520.0, 1560.0, 1700.0)
FLOOD_DEPTH_M = 0.8
FLOOD_WALKABLE_M = 1.0  # <= 1.4 (acceptance 6)


def _load(p: Path):
    return json.loads(p.read_text(encoding="utf-8"))


def _dump(p: Path, obj):
    p.write_text(json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")


def _len_in_rect(lines) -> float:
    x0, y0, x1, y1 = RECT
    tot = 0.0
    for ln in lines:
        for a, b in zip(ln, ln[1:]):
            mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
            if x0 <= mx <= x1 and y0 <= my <= y1:
                tot += ht.length([a, b])
    return tot


def run(repo: Path, out: Path, incoming: Path) -> int:
    traced_dir = incoming
    missing = []
    f_h, f_flood, f_rm = traced_dir / "hedgerows.geojson", traced_dir / "flood.geojson", traced_dir / "remove_ids.txt"
    f_edits = traced_dir / "terrain_edits.geojson"
    f_target = out / "slice_m1_targets.json"
    for name, p in (("traced hedgerows", f_h), ("flood polygons", f_flood), ("remove_ids list", f_rm)):
        if not p.is_file():
            missing.append(name)
    target = _load(f_target).get("hedgerow_total_m_target") if f_target.is_file() else None
    if target is None:
        missing.append("hedgerow_total_m_target")
    if not f_target.is_file():
        _dump(f_target, {"hedgerow_total_m_target": None, "note": "provided by a human (H-061 calibration)"})

    layers = {k: _load(out / f"{k}.geojson") for k in osm.LAYERS}
    # --- removals
    removed = []
    base = osm.find_input(repo)
    mp = _load(out / "meta.json")
    crs = mp["crs"]
    els = osm.normalise(osm.load(base), (crs["origin_e"], crs["origin_n"]))
    for e in els:
        r = modern_filter.osm_rules.modern_reason(e["tags"])
        if r:
            removed.append({"id": e["id"], "reason": f"auto draft filter: {r}"})
    rm_ids = set()
    if f_rm.is_file():
        for ln in f_rm.read_text(encoding="utf-8").splitlines():
            ln = ln.split("#")[0].strip()
            if ln:
                rm_ids.add(ln.split()[0])
        for layer in ("roads", "buildings"):
            kept = []
            for f in layers[layer]["features"]:
                if f["properties"]["_osm_id"] in rm_ids:
                    removed.append({"id": f["properties"]["_osm_id"], "reason": "human removal list (1944)"})
                else:
                    kept.append(f)
            layers[layer]["features"] = kept
    removed.sort(key=lambda d: (d["id"], d["reason"]))
    _dump(out / "removed_modern.json", removed)

    # --- hedgerows
    draft = [f for f in layers["hedgerows"]["features"] if f["properties"].get("source") == "osm_draft"]
    traced_f = []
    if f_h.is_file():
        for f in _load(f_h)["features"]:
            if f["geometry"]["type"] != "LineString":
                continue
            pts = ht.smooth([tuple(c) for c in f["geometry"]["coordinates"]])
            p = f["properties"]
            h = float(p.get("height_m", 2.0))
            g = float(p.get("max_gap_m", 0.0))
            traced_f.append({"type": "Feature", "geometry": {"type": "LineString", "coordinates": [[clean.r(x), clean.r(y)] for x, y in pts]},
                             "properties": {"height_m": h, "mound_width_m": float(p.get("mound_width_m", 1.0)), "gap": g > 0,
                                            "vehicle_blocking": ht.vehicle_blocking(h, g), "kind": "hedge", "source": "traced",
                                            "confidence": p.get("confidence", "C"), "verified": True,
                                            "_osm_id": p.get("_osm_id", "way/0"), "_derived": []}})
        kept_d, dropped = ht.merge([[tuple(c) for c in f["geometry"]["coordinates"]] for f in traced_f],
                                   [[tuple(c) for c in f["geometry"]["coordinates"]] for f in draft])
        draft = [f for f in draft if [tuple(c) for c in f["geometry"]["coordinates"]] in kept_d]
        layers["hedgerows"]["features"] = traced_f + draft
    hl = [[tuple(c) for c in f["geometry"]["coordinates"]] for f in layers["hedgerows"]["features"]]
    hed_m = round(_len_in_rect(hl), 1)

    # --- flood
    water = [f for f in layers["water"]["features"] if f["properties"]["kind"] != "flood_marsh"]
    if f_flood.is_file():
        for f in _load(f_flood)["features"]:
            if f["geometry"]["type"] != "Polygon":
                continue
            water.append({"type": "Feature", "geometry": f["geometry"],
                          "properties": {"kind": "flood_marsh", "depth_m": FLOOD_DEPTH_M, "walkable_depth_m": FLOOD_WALKABLE_M,
                                         "vehicle_blocking": True, "verified": True, "_osm_id": "relation/0", "_derived": ["depth_m", "walkable_depth_m"]}})
    layers["water"]["features"] = water
    for k in ("hedgerows", "water", "roads", "buildings"):
        _dump(out / f"{k}.geojson", layers[k])

    # --- terrain edits
    edits = _load(f_edits)["features"] if f_edits.is_file() else []
    _dump(out / "terrain_edits.geojson", {"type": "FeatureCollection", "features": edits})
    hm = out / "heightmap_1m.r16"
    n = mp["height"]["samples"]
    diff = {"cells_changed": 0, "max_abs_dh_m": 0.0, "polygons": [], "warnings": [], "exceeds_max_delta": False}
    if edits and hm.is_file():
        q = export.read_r16(hm, n)
        lo, hi = mp["height"]["hmin_m"], mp["height"]["hmax_m"]
        modern = hm.with_suffix(".modern.r16")
        if not modern.exists():
            modern.write_bytes(hm.read_bytes())
        new, diff = apply_edits.apply_terrain_edits(export.dequantize(export.read_r16(modern, n), lo, hi), edits)
        export.write_r16(hm, export.quantize(new, lo, hi))
    _dump_pretty(repo / "reports" / "m1-heightmap-diff.json", diff)

    dev = None if not target else round((hed_m - target) / target * 100, 1)
    status = "partial" if missing else "complete"
    rep = {"schema": 1, "status": status, "missing_inputs": missing,
           "hedgerow_slice_total_m": hed_m, "hedgerow_target_m": target, "hedgerow_deviation_pct": dev,
           "hedgerow_sources": {s: sum(1 for f in layers["hedgerows"]["features"] if f["properties"].get("source") == s) for s in ("traced", "osm_draft")},
           "flood_marsh_polygons": sum(1 for f in water if f["properties"]["kind"] == "flood_marsh"),
           "removed_modern": len(removed), "removed_by_human_list": len(rm_ids), "terrain_edits": len(edits)}
    _dump_pretty(repo / "reports" / "m1-1944-edits.json", rep)
    print(json.dumps(rep, ensure_ascii=False))
    return EXIT_PARTIAL if missing else 0


def _dump_pretty(p: Path, obj):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n", encoding="utf-8")
