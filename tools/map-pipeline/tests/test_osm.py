import json
import subprocess
import sys
from pathlib import Path

import pytest

from map_pipeline import clean, geo, modern_filter, osm, osm_rules

ROOT = Path(__file__).resolve().parents[3]
ORIGIN = (626432, 5462016)


def ll(x, y):
    e, n = ORIGIN[0] + x, ORIGIN[1] + y
    lat, lon = geo.from_utm(e, n)
    return {"lat": lat, "lon": lon}


def way(i, pts, **tags):
    return {"type": "way", "id": i, "tags": tags, "geometry": [ll(*p) for p in pts]}


SQ = [(100, 100), (120, 100), (120, 120), (100, 120), (100, 100)]
FIX = {"elements": [
    way(1, [(10, 10), (500, 10)], highway="tertiary"),
    way(2, [(10, 50), (400, 50)], highway="track", width="3.5"),
    way(3, [(10, 90), (100, 90)], highway="footway"),
    way(4, [(300, 10), (300, 200)], highway="secondary", bridge="yes"),
    way(5, [(10, 300), (900, 300)], railway="rail"),
    way(6, [(10, 400), (800, 400)], waterway="canal"),
    way(7, [(500, 500), (540, 500), (540, 540), (500, 540), (500, 500)], natural="water"),
    way(8, [(10, 600), (300, 600)], barrier="hedge", height="2.2"),
    way(9, [(10, 650), (300, 650)], natural="tree_row"),
    way(10, SQ, building="house", **{"building:levels": "2"}),
    way(11, [(200, 100), (202, 100), (202, 101), (200, 101), (200, 100)], building="yes"),
    way(12, [(700, 700), (800, 700), (800, 800), (700, 800), (700, 700)], building="retail"),
    way(13, [(900, 900), (950, 900), (950, 950), (900, 950), (900, 900)], landuse="forest"),
    way(14, [(2000, 1000), (2100, 1000)], highway="residential"),
    way(15, [(3000, 3000), (3100, 3000)], highway="residential"),
    way(16, [(10, 10), (500, 10)], highway="tertiary"),
    way(17, [(10, 700), (60, 700), (10, 760), (60, 760), (10, 700)], building="yes"),
    way(18, [(10, 800), (90, 800)], barrier="wall"),
]}


@pytest.fixture(scope="module")
def res():
    return osm.extract(FIX, ORIGIN)


def feats(res, layer):
    return {f["properties"]["_osm_id"]: f["properties"] for f in res[0][layer]}


def test_roads_rules(res):
    r = feats(res, "roads")
    assert r["way/1"]["class"] == "paved" and r["way/1"]["width_m"] == 6.0 and "width_m" in r["way/1"]["_derived"]
    assert r["way/2"]["class"] == "dirt" and r["way/2"]["width_m"] == 3.5 and r["way/2"]["vehicle_ok"]
    assert r["way/2"]["_derived"] == ["surface"]
    assert r["way/3"]["class"] == "path" and not r["way/3"]["vehicle_ok"]
    assert r["way/4"]["bridge"] and r["way/4"]["elev_mode"] == "bridge"
    assert r["way/5"]["class"] == "rail" and not r["way/5"]["vehicle_ok"]


def test_water_hedge_building_forest(res):
    w = feats(res, "water")
    assert w["way/6"]["kind"] == "canal" and w["way/6"]["walkable_depth_m"] == 0.5
    assert w["way/7"]["kind"] == "pond"
    h = feats(res, "hedgerows")
    assert h["way/8"]["height_m"] == 2.2 and "height_m" not in h["way/8"]["_derived"] and h["way/8"]["vehicle_blocking"]
    assert h["way/9"]["height_m"] == 2.0 and "height_m" in h["way/9"]["_derived"] and h["way/9"]["gap"] is False
    b = feats(res, "buildings")
    assert b["way/10"]["levels"] == 2 and not b["way/10"]["enterable"] and b["way/10"]["style"] == "unknown"
    assert "levels" not in feats(res, "buildings")["way/10"]["_derived"]
    assert "way/12" not in b  # building=retail is a modern drop
    assert feats(res, "forests")["way/13"]["kind"] == "forest"


def test_filters_and_clean(res):
    st = res[1]
    assert st["dropped_modern[building=retail]"] == 1
    assert st["dropped_small"] == 1            # way 11 is 2 m²
    assert st["dropped_self_intersect"] == 1   # bow-tie way 17
    assert st["duplicates"] == 1               # way 16 repeats way 1
    assert st["skipped_wall_fence"] == 1
    assert "way/11" not in feats(res, "buildings")
    assert "way/17" not in feats(res, "buildings")


def test_clip_and_range(res):
    r = res[0]["roads"]
    p14 = [f for f in r if f["properties"]["_osm_id"] == "way/14"][0]
    xs = [c[0] for c in p14["geometry"]["coordinates"]]
    assert max(xs) == 2048.0
    assert not [f for f in r if f["properties"]["_osm_id"] == "way/15"]
    for layer in res[0].values():
        for f in layer:
            assert all(0 <= x <= 2048 and 0 <= y <= 2048 for x, y in osm.iter_coords(f))


def test_roundtrip_error_under_10cm():
    pts = osm._local([ll(123.456, 789.012)], ORIGIN)[0]
    assert abs(pts[0] - 123.456) < 0.1 and abs(pts[1] - 789.012) < 0.1


def test_clip_helpers():
    assert clean.clip_line([(-10, 5), (10, 5)]) == [[(0.0, 5.0), (10, 5)]] or clean.clip_line([(-10, 5), (10, 5)])[0][0] == (0.0, 5.0)
    ring = clean.clip_ring([(-10, -10), (10, -10), (10, 10), (-10, 10)])
    assert abs(clean.area(ring)) == 100
    assert clean.self_intersects([(0, 0), (10, 10), (10, 0), (0, 10)])
    assert not clean.self_intersects([(0, 0), (10, 0), (10, 10), (0, 10)])


def test_multipolygon_join():
    a = [(0, 0), (10, 0), (10, 5)]
    b = [(10, 5), (10, 10), (0, 10), (0, 0)]
    rings = osm._join_rings([a, b])
    assert len(rings) == 1 and rings[0][0] == rings[0][-1]


def test_modern_filter():
    kept, dropped = modern_filter.split([{"tags": {"landuse": "retail"}}, {"tags": {"highway": "x"}}])
    assert len(kept) == 1 and dropped == {"landuse=retail": 1}


def test_deterministic_output(tmp_path):
    a = osm.dump_layer(tmp_path / "a.geojson", osm.extract(FIX, ORIGIN)[0]["roads"])
    b = osm.dump_layer(tmp_path / "b.geojson", osm.extract(FIX, ORIGIN)[0]["roads"])
    assert a == b


def test_missing_input_exit_code(tmp_path):
    p = subprocess.run([sys.executable, "-m", "map_pipeline", "osm", "--repo", str(tmp_path), "--out", str(tmp_path)],
                       cwd=ROOT / "tools/map-pipeline", capture_output=True, text=True)
    assert p.returncode == 3 and "BLOCKED" in p.stderr


def test_real_data_acceptance():
    d = ROOT / "data/maps/carentan"
    s = json.loads((ROOT / "reports/m1-osm-summary.json").read_text())
    assert s["out_of_range_vertices"] == 0
    assert sum(s["road_length_m"].values()) >= 1000
    assert s["slice_m1"]["buildings"] >= 50
    assert s["intersection_utm_check"]["ok"] and s["intersection_utm_check"]["sampled"] >= 10
    assert s["intersection_utm_check"]["max_err_m"] < 0.1
    assert "© OpenStreetMap contributors" in (d / "ATTRIBUTION_OSM.txt").read_text(encoding="utf-8")
    assert "ODbL" in (d / "LICENSE_NOTICE.md").read_text(encoding="utf-8")
    meta = json.loads((d / "meta.json").read_text(encoding="utf-8"))
    osm_src = [src for src in meta["sources"] if src["id"] == "osm"][0]
    assert osm_src.get("inputs") and len(osm_src["inputs"][0]["sha256"]) == 64
    assert osm_src.get("osm_timestamp") == s["osm_timestamp"]
    for k in [k for k in osm.LAYERS if k != 'buildings']:  # buildings are annotated later (M1-034)
        import hashlib
        assert hashlib.sha256((d / f"{k}.geojson").read_bytes()).hexdigest() == s["layer_sha256"][k]
    for layer in osm.LAYERS:
        for f in json.loads((d / f"{layer}.geojson").read_text())["features"]:
            if f["geometry"]["type"] == "Polygon":
                assert abs(clean.area(f["geometry"]["coordinates"][0][:-1])) >= 4.0


def test_extend_dangles_closes_small_gap():
    roads = [
        {"type": "Feature", "properties": {"vehicle_ok": True, "_osm_id": "way/1"},
         "geometry": {"type": "LineString", "coordinates": [[100.0, 100.0], [200.0, 100.0]]}},
        {"type": "Feature", "properties": {"vehicle_ok": True, "_osm_id": "way/2"},
         "geometry": {"type": "LineString", "coordinates": [[200.8, 100.0], [300.0, 100.0]]}},
        {"type": "Feature", "properties": {"vehicle_ok": True, "_osm_id": "way/3"},
         "geometry": {"type": "LineString", "coordinates": [[310.0, 100.0], [400.0, 100.0]]}},
    ]
    n = clean.extend_dangles(roads, max_gap_m=1.5)
    assert n == 1
    assert roads[0]["geometry"]["coordinates"][-1] == [200.8, 100.0] or roads[1]["geometry"]["coordinates"][0] == [200.0, 100.0]
    assert roads[2]["geometry"]["coordinates"][0] == [310.0, 100.0]


def test_check_roads_cli():
    from map_pipeline.cli import main
    assert main(["check-roads", str(ROOT / "data/maps/carentan")]) == 0
