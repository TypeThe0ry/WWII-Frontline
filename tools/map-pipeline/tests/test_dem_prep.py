import numpy as np
import pytest

from map_pipeline import dem_prep as dp, export, qa, slice_m1 as sl


def test_quantize_round_trip_error_below_half_step():
    rng = np.random.default_rng(1)
    h = rng.uniform(-1.2, 45.0, 10_000)
    q = export.quantize(h, -1.2, 45.0)
    err = np.abs(h - export.dequantize(q, -1.2, 45.0)).max()
    assert err <= (45.0 + 1.2) / 65535 / 2 + 1e-6 and err < 1e-3


def test_file_sizes_match_acceptance():
    assert sl.NEAR_N * sl.NEAR_N * 2 == 8_396_802
    assert sl.FAR_N * sl.FAR_N * 2 == 2_101_250


def _fixture():
    rng = np.random.default_rng(7)
    near = rng.uniform(0, 20, (sl.NEAR_N, sl.NEAR_N))
    far = rng.uniform(0, 20, (sl.FAR_N, sl.FAR_N))
    return near, far


def test_slice_keeps_the_rectangle_exact_and_far_data_outside():
    near, far = _fixture()
    out, w = sl.blend(near, far)
    x0, y0, x1, y1 = (int(v) for v in sl.RECT_DESIGN)
    assert np.array_equal(out[y0 : y1 + 1, x0 : x1 + 1], near[y0 : y1 + 1, x0 : x1 + 1])
    up = sl.far_to_near_grid(far)
    # >= 32 m outside the rectangle the result is exactly the upsampled far ring.
    assert np.array_equal(out[: y0 - 32, :], up[: y0 - 32, :])
    assert np.array_equal(out[:, x1 + 32 :], up[:, x1 + 32 :])
    assert set(np.unique(w[y0 : y1 + 1, x0 : x1 + 1])) == {1.0}


def test_feather_is_monotonic_and_spans_32m():
    w = sl.mask()
    row = w[1000, 380 - 40 : 380 + 1]  # walking east into the rectangle
    assert np.all(np.diff(row) >= 0)
    assert row[0] == 0.0 and row[-1] == 1.0
    assert w[1000, 380 - 16] == pytest.approx(0.5)


def test_far_upsample_hits_far_samples_exactly():
    far = np.arange(sl.FAR_N * sl.FAR_N, dtype=np.float64).reshape(sl.FAR_N, sl.FAR_N)
    up = sl.far_to_near_grid(far)
    # local x = 0 is far index 384 (= 3072 / 8); x = 8 is 385.
    assert up[0, 0] == far[384, 384] and up[8, 16] == far[385, 386]


def test_blocker_polygon_is_a_4m_band():
    b = sl.blocker_polygon()
    assert b["outer"][0] == [380.0, 520.0] and b["inner"][0] == [384.0, 524.0]


def test_fill_nodata_counts_and_blends_neighbours():
    h = np.full((50, 50), 10.0)
    m = np.zeros_like(h, bool)
    m[20:23, 20:23] = True
    h[m] = -9999
    out, n = dp.fill_nodata(h, m)
    assert n == 9 and np.allclose(out, 10.0)


def test_fill_nodata_over_one_percent_is_an_error():
    h = np.zeros((20, 20))
    m = np.zeros_like(h, bool)
    m[:5, :] = True
    with pytest.raises(ValueError, match="no-data"):
        dp.fill_nodata(h, m)


def test_round_range_goes_outward():
    assert dp.round_range(-1.17, 44.93) == (-1.2, 45.0)
    assert dp.round_range(0.0, 0.0) == (0.0, 0.1)


def test_datum_offset_prefers_water_mask():
    h = np.array([[5.0, 0.4], [3.0, 9.0]])
    assert dp.datum_offset(h, np.array([[False, True], [False, False]])) == 0.4
    assert dp.datum_offset(h) <= 3.0


def test_input_manifest_is_sorted_and_hashed(tmp_path):
    (tmp_path / "b.tif").write_bytes(b"b")
    (tmp_path / "a.tif").write_bytes(b"a")
    (tmp_path / "x.txt").write_bytes(b"x")
    files = dp.find_inputs(tmp_path)
    assert [f.name for f in files] == ["a.tif", "b.tif"]
    m = dp.input_manifest(files)
    assert m[0]["file"] == "a.tif" and len(m[0]["sha256"]) == 64
    assert dp.find_inputs(tmp_path / "nope") == []


def test_qa_thresholds():
    pts = [{"id": str(i), "e": 0, "n": 0, "h_a": 5.0, "h_b": 5.0 + 0.1 * i} for i in range(10)]
    r = qa.compare(pts)
    assert r["ok"] and r["max_abs_m"] == pytest.approx(0.9)
    pts[3]["h_b"] = 9.0
    assert not qa.compare(pts)["ok"]
    with pytest.raises(ValueError):
        qa.compare(pts[:5])


def test_qa_csv_has_header_and_rows(tmp_path):
    pts = [{"id": str(i), "e": 1, "n": 2, "h_a": 1.0, "h_b": 1.5} for i in range(10)]
    qa.write_csv(tmp_path / "c.csv", pts)
    lines = (tmp_path / "c.csv").read_text().splitlines()
    assert len(lines) == 11 and lines[0].startswith("id,e,n")


def test_dem_command_exits_3_without_a_raster(tmp_path, capsys):
    from map_pipeline.cli import main

    rc = main(["dem", "--incoming", str(tmp_path / "none"), "--out", str(tmp_path), "--repo", str(tmp_path)])
    assert rc == dp.EXIT_MISSING_DEM
    err = capsys.readouterr().err
    assert "missing" in err and "LiDAR HD" in err


def test_dem_command_refuses_unverified_sources(tmp_path):
    import json
    from map_pipeline.cli import main

    inc = tmp_path / "dem"
    inc.mkdir()
    (inc / "t.tif").write_bytes(b"x")
    (tmp_path / "meta.json").write_text(json.dumps({"sources": [
        {"id": "ign_lidar_hd", "asset_id": "AST-M-LIDAR-01", "archive": "a"},
        {"id": "ign_rge_alti", "asset_id": "AST-M-ALTI-01", "archive": "b"}]}))
    assert main(["dem", "--incoming", str(inc), "--out", str(tmp_path), "--repo", str(tmp_path)]) == dp.EXIT_SOURCES_NOT_OK
