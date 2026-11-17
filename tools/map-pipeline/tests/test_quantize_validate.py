import json
import shutil

import numpy as np
import pytest

from map_pipeline.dem import window_bounds, GdalMissing, require_gdal
from map_pipeline.export import dequantize, quantize
from map_pipeline.cli import main
from map_pipeline.validate import validate


def test_quantize_roundtrip_error():
    rng = np.random.default_rng(0)
    h = rng.uniform(-3.0, 40.0, 100000)
    q = quantize(h, -3.0, 40.0)
    assert np.abs(dequantize(q, -3.0, 40.0) - h).max() <= 43.0 / 65535 / 2 + 1e-12
    assert quantize(np.array([-3.0, 40.0]), -3.0, 40.0).tolist() == [0, 65535]


def test_window_bounds():
    assert window_bounds(1000.0, 2000.0) == (999.5, 1999.5, 1256.5, 2256.5)
    assert window_bounds(0, 0, 8.0, 1025)[2] == 8192 + 4.0


def test_validate_ok(m0_dir):
    assert validate(m0_dir) == []
    assert main(["validate", str(m0_dir)]) == 0


@pytest.fixture
def copy(m0_dir, tmp_path):
    d = tmp_path / "m0"
    shutil.copytree(m0_dir, d)
    return d


def test_validate_truncated_heightmap(copy):
    p = copy / "heightmap_1m.r16"
    p.write_bytes(p.read_bytes()[:-2])
    assert validate(copy)
    assert main(["validate", str(copy)]) == 1


def test_validate_truncated_chunk(copy):
    p = copy / "chunks" / "b_1_1.bin"
    p.write_bytes(p.read_bytes()[:-2])
    assert any("b_1_1" in e for e in validate(copy))


def test_validate_seam_break(copy):
    p = copy / "chunks" / "b_0_0.bin"
    a = np.frombuffer(p.read_bytes(), dtype="<u2").copy().reshape(65, 65)
    a[10, 64] ^= 0x40
    p.write_bytes(a.tobytes())
    errs = validate(copy)
    assert any("seam" in e for e in errs)


def test_validate_spawn_out_of_bounds(copy):
    sp = json.loads((copy / "spawns.json").read_text())
    sp["hq"]["US"]["pos"] = [300, 56]
    (copy / "spawns.json").write_text(json.dumps(sp))
    assert any("outside" in e for e in validate(copy))


def test_validate_spawn_on_steep_slope(copy):
    sp = json.loads((copy / "spawns.json").read_text())
    sp["hq"]["US"]["pos"] = [128, 128]  # middle of the embankment ramp
    (copy / "spawns.json").write_text(json.dumps(sp))
    assert any("slope" in e for e in validate(copy))


def test_dem_path_clear_error_without_gdal():
    try:
        require_gdal()
    except GdalMissing as e:
        assert "GDAL" in str(e)
    else:
        pytest.skip("GDAL present")


def test_dem_path_cli_error_without_gdal(tmp_path):
    try:
        require_gdal()
        pytest.skip("GDAL present")
    except GdalMissing:
        pass
    assert main(["m0", "--out", str(tmp_path / "x"), "--dem", "nope.tif", "--window", "0,0"]) == 1


def test_dem_synthetic_geotiff(tmp_path):
    try:
        gdal = require_gdal()
    except GdalMissing:
        pytest.skip("GDAL not installed")
    from map_pipeline.dem import load_dem

    # 1 m grid, 9000 m square so the far window is covered; plane h = 0.01*E
    n = 9000
    path = str(tmp_path / "t.tif")
    ds = gdal.GetDriverByName("GTiff").Create(path, n, n, 1, gdal.GDT_Float32)
    ds.SetGeoTransform((600000.0, 1.0, 0, 5460000.0 + n, 0, -1.0))
    xs = np.arange(n, dtype=np.float32) + 0.5
    ds.GetRasterBand(1).WriteArray(np.tile(0.01 * xs, (n, 1)))
    ds = None
    near, far = load_dem(path, 604000.0, 5464000.0)
    assert near.shape == (257, 257) and far.shape == (1025, 1025)
    assert abs(near[0, 0] - 0.01 * 4000.0) < 0.02
    assert abs(near[0, 256] - near[0, 0] - 2.56) < 0.02
