import numpy as np
import pytest

from map_pipeline import buildings_annotate as ba, zones as zn


def grid(blocked_boxes=(), n=200, res=2.0):
    b = np.zeros((n, n), dtype=bool)
    for x0, y0, x1, y1 in blocked_boxes:
        b[int(y0 // res):int(y1 // res), int(x0 // res):int(x1 // res)] = True
    return zn.Grid(b, res)


def test_snap_within_and_beyond():
    g = grid([(100, 100, 140, 140)])
    (p, d) = g.snap(120, 120)
    assert 0 < d <= zn.SNAP_MAX_M and g.standable(*p)
    assert g.snap(120, 120, max_m=5.0) is None  # fixture: > limit fails


def test_fixture_zone_in_water_or_building_fails():
    g = grid([(90, 90, 110, 110)])
    assert not zn.rule_not_blocked(g, (100, 100))
    assert zn.rule_not_blocked(g, (150, 150))


def _ring_wall(gaps_deg):
    import math
    g = grid()
    for r in range(g.n):
        for c in range(g.n):
            x, y = (c + .5) * 2, (r + .5) * 2
            d = math.hypot(x - 100, y - 100)
            ang = math.degrees(math.atan2(y - 100, x - 100)) % 360
            if 36 <= d < 42 and not any(abs((ang - a + 180) % 360 - 180) < 4 for a in gaps_deg):
                g.b[r, c] = True
    return g


def test_fixture_two_entrances_fails():
    # circular wall at r=36..42 m with openings; entrances counted on the R=40 rim
    assert zn.rule_entrances(_ring_wall([15, 195]), (100, 100), 40) == 2
    assert zn.rule_entrances(_ring_wall([15, 105, 195, 285]), (100, 100), 40) >= zn.MIN_ENTRANCES
    assert zn.rule_entrances(grid(), (100, 100), 40) == zn.SECTORS


def test_fixture_hq_sealed_fails():
    g = grid([(40, 40, 160, 44), (40, 156, 160, 160), (40, 40, 44, 160), (156, 40, 160, 160)])
    assert zn.infantry_path(g, (100, 100), (30, 30), rect=(0, 0, 200, 200), margin=0) is None
    assert zn.infantry_path(grid(), (100, 100), (30, 30), rect=(0, 0, 200, 200), margin=0) is not None


def test_road_graph_route_and_width():
    def road(i, pts, w, br=False):
        return {"geometry": {"coordinates": pts}, "properties": {"vehicle_ok": True, "width_m": w, "bridge": br, "_osm_id": f"way/{i}"}}
    rg = zn.RoadGraph([road(1, [[0, 0], [100, 0]], 6.0), road(2, [[100, 0], [200, 0]], 3.0, True)])
    r = zn.route_summary(rg, [(0, 0), (200, 0)])
    assert r["min_width_m"] == 3.0 and r["chokepoints"] == ["way/2"] and r["length_m"] == 200.0


def test_enterable_cap_and_cover_fallback():
    def b(i, x, y):
        ring = [[x, y], [x + 10, y], [x + 10, y + 10], [x, y + 10], [x, y]]
        return {"geometry": {"type": "Polygon", "coordinates": [ring]}, "properties": {"_osm_id": f"way/{i}", "levels": 1, "_derived": ["levels"], "type": "yes"}}
    bs = [b(i, 400 + (i % 10) * 12, 600 + (i // 10) * 12) for i in range(40)]
    zs = [{"id": "L2a", "pos": [450, 650], "radius_m": 200}]
    st = ba.annotate(bs, zs)
    assert st["enterable"] <= 0.25 * st["in_slice"]
    cp = zn.cover_points(grid(), (100, 100), 30, [], [])
    assert len(cp) >= zn.MIN_COVER and all(c["kind"] == "placeholder_ring" for c in cp)


def test_real_slice_all_rules_pass():
    import json
    from pathlib import Path
    r = json.loads((Path(__file__).resolve().parents[3] / "reports/m1-zones-validate.json").read_text())
    assert r["ok"], [k for k, v in r["rules"].items() if not v["pass"]]
    assert r["rules"]["garrison_hq_dist>=200m"]["pass"]


def test_door_for_prefers_open_exterior_over_adjacent_building():
    # Building A: [100,100]-[110,100]-[110,110]-[100,110]; road is to the west (x=95),
    # but a neighbouring building blocks the west side (90..100, 100..110).
    ring = [[100.0, 100.0], [110.0, 100.0], [110.0, 110.0], [100.0, 110.0], [100.0, 100.0]]
    g = grid([(90, 100, 110, 110)], n=200, res=1.0)
    road = [[[92.0, 90.0], [92.0, 120.0]]]
    d = ba.door_for(ring, road, grid=g)
    assert d is not None and d["exterior_clear"] is True
    # Must NOT pick the blocked west wall (x=100, y=105)
    assert d["center"] != [100.0, 105.0]


def test_garrison_too_close_fails_validate(tmp_path):
    import json, shutil
    from pathlib import Path
    src = Path(__file__).resolve().parents[3] / "data/maps/carentan"
    dst = tmp_path / "carentan"
    dst.mkdir()
    for fn in ("buildings.geojson", "roads.geojson", "water.geojson", "landmarks.json", "vehicle_routes.json"):
        shutil.copy2(src / fn, dst / fn)
    shutil.copytree(src / "m1", dst / "m1")
    sp_path = dst / "m1" / "spawns.json"
    sp = json.loads(sp_path.read_text())
    sp["garrison_candidates"]["US"]["pos"] = [sp["hq"]["US"]["pos"][0] + 50.0, sp["hq"]["US"]["pos"][1]]
    sp_path.write_text(json.dumps(sp))
    rep, ok = zn.validate_slice(tmp_path, dst)
    assert not ok and not rep["rules"]["garrison_hq_dist>=200m"]["pass"]
