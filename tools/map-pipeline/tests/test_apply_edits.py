import numpy as np
import pytest

from map_pipeline import apply_edits as ae, hedgerow_tools as ht


def circle(cx, cy, r, n=64):
    import math
    return [[cx + r * math.cos(2 * math.pi * k / n), cy + r * math.sin(2 * math.pi * k / n)] for k in range(n)]


def feat(i, mode, ring, order=0, **kw):
    return {"type": "Feature", "properties": {"id": i, "mode": mode, "order": order, "reason": "t", **kw},
            "geometry": {"type": "Polygon", "coordinates": [ring + [ring[0]]]}}


def test_raise_feather():
    h = np.zeros((101, 101))
    out, rep = ae.apply_terrain_edits(h, [feat("a", "raise", circle(50, 50, 10), delta_m=2.0, transition_m=10)])
    assert abs(out[50, 50] - 2.0) < 0.01
    assert out[50, 71] == 0.0 and out[50, 29] == 0.0  # beyond 10 m outside the ring
    prof = out[50, 60:71]
    assert np.all(np.diff(prof) <= 1e-12)  # monotone falloff
    assert rep["cells_changed"] > 0 and rep["max_abs_dh_m"] == pytest.approx(2.0, abs=0.01)


def test_overlap_order():
    h = np.zeros((60, 60))
    a = feat("a", "set_height", circle(30, 30, 12), order=2, target_h_m=5.0)
    b = feat("b", "set_height", circle(30, 30, 20), order=1, target_h_m=1.0)
    o1, _ = ae.apply_terrain_edits(h, [a, b])
    assert o1[30, 30] == pytest.approx(5.0) and o1[30, 45] == pytest.approx(1.0)
    c = feat("c", "raise", circle(30, 30, 8), order=3, delta_m=1.0)
    o2, _ = ae.apply_terrain_edits(h, [a, b, c])
    assert o2[30, 30] == pytest.approx(6.0)
    o3, _ = ae.apply_terrain_edits(h, [c, b, a])  # list order irrelevant
    assert np.array_equal(o2, o3)


def test_modes_and_idempotent():
    h = np.tile(np.linspace(0, 4, 50), (50, 1))
    e = [feat("f", "flatten", circle(25, 25, 8)), feat("e", "embankment", circle(40, 10, 5), target_h_m=9.0),
         feat("l", "lower", circle(10, 40, 5), delta_m=0.3)]
    o1, _ = ae.apply_terrain_edits(h, e)
    o2, _ = ae.apply_terrain_edits(h, e)
    assert o1.tobytes() == o2.tobytes()
    assert o1[10, 40] == pytest.approx(9.0) and o1[40, 10] == pytest.approx(h[40, 10] - 0.3)
    assert np.ptp(o1[25, 20:30]) < 1e-9


def test_no_edits_and_warning():
    h = np.ones((10, 10))
    o, rep = ae.apply_terrain_edits(h, [])
    assert np.array_equal(o, h) and rep["cells_changed"] == 0
    f = feat("w", "raise", circle(5, 5, 3), delta_m=4.0)
    f["properties"]["reason"] = ""
    _, rep = ae.apply_terrain_edits(h, [f])
    assert rep["warnings"]


def test_vehicle_blocking_boundaries():
    assert not ht.vehicle_blocking(1.79, 0)
    assert ht.vehicle_blocking(1.81, 0)
    assert ht.vehicle_blocking(2.0, 3.9)
    assert not ht.vehicle_blocking(2.0, 4.1)


def test_resample_length_and_merge():
    line = [(0, 0), (30, 0), (30, 40)]
    rs = ht.resample(line, 2.0)
    assert abs(ht.length(rs) - 70) / 70 < 0.01
    sm = ht.smooth(line)
    assert ht.length(sm) < 70
    g = ht.gaps([[(0, 0), (10, 0)], [(13.5, 0), (30, 0)]])
    assert g == [3.5]
    kept, dropped = ht.merge([[(0, 0), (50, 0)]], [[(5, 1), (40, 2)], [(5, 20), (40, 20)]])
    assert dropped == 1 and len(kept) == 1


def test_edits_report_partial_without_human_inputs():
    import json
    from pathlib import Path
    root = Path(__file__).resolve().parents[3]
    r = json.loads((root / "reports/m1-1944-edits.json").read_text())
    assert r["status"] == "partial" and "traced hedgerows" in r["missing_inputs"]
    assert r["hedgerow_sources"]["traced"] == 0  # no OSM-draft hedges posing as traced
    assert r["out_of_range_vertices"] == 0
    assert all(d["reason"] for d in json.loads((root / "data/maps/carentan/removed_modern.json").read_text()))
    d = json.loads((root / "reports/m1-heightmap-diff.json").read_text())
    assert d["cells_changed"] == 0 and d["polygons"] == []


def test_full_2049_grid_20_polygons_under_30s():
    import time
    h = np.zeros((2049, 2049), dtype=np.float64)
    edits = [feat(f"p{i}", "raise", circle(200 + (i % 5) * 380, 200 + (i // 5) * 420, 45, n=16),
                  order=i, delta_m=1.2, transition_m=12.0) for i in range(20)]
    t0 = time.perf_counter()
    o1, rep = ae.apply_terrain_edits(h, edits)
    dt = time.perf_counter() - t0
    assert dt <= 30.0, f"2049x2049 x 20 polygons took {dt:.2f}s (> 30s)"
    assert rep["cells_changed"] > 100_000 and rep["max_abs_dh_m"] == pytest.approx(1.2, abs=0.01)
    o2, _ = ae.apply_terrain_edits(h, edits)
    assert o1.tobytes() == o2.tobytes()
