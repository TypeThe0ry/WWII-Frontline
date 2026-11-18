import hashlib
import math
import subprocess
import sys
from pathlib import Path

import pytest

from map_pipeline import anchors as an, carentan, geo

ROOT = Path(__file__).resolve().parents[3]


def _fixture(dx, dy, rot_deg):
    """Eight design points, real = design moved by (dx, dy, rot) about the pivot."""
    pts = [(300, 300), (1700, 400), (1000, 1000), (400, 1600), (1600, 1700),
           (1277, 897), (900, 1450), (200, 900)]
    sim = {"dx": dx, "dy": dy, "rot_deg": rot_deg}
    real = an.apply(pts, sim)
    return [an.Anchor(f"a{i}", p, r, 50.0) for i, (p, r) in enumerate(zip(pts, real))]


def test_anchor_count():
    meta = carentan.build_meta(ROOT, "")
    assert len(meta["anchors"]) >= 8
    assert sum(1 for a in meta["anchors"] if a["confidence_m"] <= 150) >= 3
    assert all(a["confidence_m"] > 0 and a["source"] for a in meta["anchors"])
    assert carentan.validate_meta(meta) == []


def test_small_offset_is_translation_only():
    sim = an.fit_similarity(_fixture(100, 0, 0))
    assert sim["rot_deg"] == 0.0 and sim["mode"] == "translate"
    assert sim["dx"] == pytest.approx(100, abs=1e-6)
    assert sim["applied"] is False


def test_large_offset_with_rotation_is_fitted():
    sim = an.fit_similarity(_fixture(220, 0, 6))
    assert 5.5 <= sim["rot_deg"] <= 6.5
    assert sim["rms_m"] <= 20
    assert sim["mode"] == "rotate"


def test_rotation_over_ten_degrees_is_an_error():
    with pytest.raises(an.AnchorError, match="exceeds"):
        an.fit_similarity(_fixture(220, 0, 12))


def test_apply_keeps_before_and_after_and_respects_applied_flag():
    sim = {"dx": 10.0, "dy": -5.0, "rot_deg": 0.0, "applied": False}
    items = {"p": (100.0, 200.0)}
    off = an.apply_to_design(items, sim)[0]
    assert off["after"] == [100.0, 200.0] and not off["moved"]
    on = an.apply_to_design(items, {**sim, "applied": True})[0]
    assert on["before"] == [100.0, 200.0] and on["after"] == [110.0, 195.0] and on["moved"]


def test_snap_moves_a_water_point_to_land_within_60m():
    water = lambda x, y: math.hypot(x - 500, y - 500) < 30  # noqa: E731  a 30 m pond
    r = an.snap_to_standable((505, 500), lambda x, y: not water(x, y))
    assert 0 < r["dist_m"] <= 60 and r["note"]
    assert not water(*r["pos"])


def test_snap_beyond_60m_is_an_error():
    lake = lambda x, y: math.hypot(x - 500, y - 500) < 200  # noqa: E731
    with pytest.raises(an.AnchorError, match="60"):
        an.snap_to_standable((500, 500), lambda x, y: not lake(x, y))


def test_utm_round_trip_is_sub_centimetre():
    for lat, lon in [(49.30679, -1.24659), (49.3, -1.3), (49.4, -1.1)]:
        e, n = geo.to_utm(lat, lon)
        la, lo = geo.from_utm(e, n)
        e2, n2 = geo.to_utm(la, lo)
        assert math.hypot(e - e2, n - n2) < 0.01
        assert abs(la - lat) < 1e-7 and abs(lo - lon) < 1e-7
    assert geo.from_local(*geo.to_local(627709, 5462913, 626432, 5462016), 626432, 5462016) == (627709, 5462913)


def test_utm_matches_maps_corner_table():
    # MAPS §1.3 corners (5-decimal degrees ~ 1 m).
    for (e, n), (lat, lon) in [((626432, 5462016), (49.29780, -1.26100)),
                               ((628480, 5464064), (49.31578, -1.23219)),
                               ((627456, 5463040), (49.30679, -1.24659))]:
        ge, gn = geo.to_utm(lat, lon)
        assert math.hypot(ge - e, gn - n) < 2.0


def test_utm_agrees_with_pyproj_when_available():
    pyproj = pytest.importorskip("pyproj")
    t = pyproj.Transformer.from_crs(4326, 32630, always_xy=True)
    e, n = t.transform(-1.24659, 49.30679)
    ge, gn = geo.to_utm(49.30679, -1.24659)
    assert math.hypot(e - ge, n - gn) < 0.001


def test_carentan_meta_is_deterministic_and_church_drives_a_tiny_translation():
    a = carentan.build_meta(ROOT, "")
    assert a == carentan.build_meta(ROOT, "")
    sim = a["similarity"]
    assert sim["applied"] is False and sim["rot_deg"] == 0 and sim["fit_anchors"] == ["church_notre_dame"]
    assert math.hypot(sim["dx"], sim["dy"]) < 5  # OSM church vs MAPS catalogue position
    church = next(x for x in a["anchors"] if x["id"] == "church_notre_dame")
    assert church["source"].startswith("osm:") and church["design_deviation_m"] < 5


def test_wrong_design_positions_are_flagged_not_fitted():
    rows = {r["id"]: r for r in carentan.resolve_anchors(ROOT)}
    assert rows["canal_lock"]["design_revised"] and rows["place_republique"]["design_revised"]
    assert not rows["pont_de_verdun"]["design_independent"]  # no independent design position
    assert all(r["id"] == "church_notre_dame" for r in carentan.fit_set(list(rows.values())))


def test_cli_writes_identical_meta_twice(tmp_path):
    def run():
        subprocess.run([sys.executable, "-m", "map_pipeline", "carentan-meta", "--repo", str(ROOT),
                        "--out", str(tmp_path / "m")], check=True, cwd=ROOT / "tools/map-pipeline",
                       capture_output=True)
        return hashlib.sha256((tmp_path / "m" / "meta.json").read_bytes()).hexdigest()

    assert run() == run()
