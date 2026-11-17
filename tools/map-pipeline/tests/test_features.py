import numpy as np

from map_pipeline.export import dequantize, read_r16, slope_deg
import json


def _slope(m0_dir):
    meta = json.loads((m0_dir / "meta.json").read_text())
    q = read_r16(m0_dir / "heightmap_1m.r16", 257)
    h = dequantize(q, meta["height"]["hmin_m"], meta["height"]["hmax_m"])
    return h, slope_deg(h)


def test_relief_range(m0_dir):
    h, _ = _slope(m0_dir)
    assert 8.0 <= h.max() - h.min() <= 12.5


def test_steep_embankment_8m_run(m0_dir):
    _, s = _slope(m0_dir)
    steep = s > 35.0
    # run along z (crossing the embankment): >= 8 consecutive samples in one column
    best_z = 0
    for x in range(257):
        run = 0
        for z in range(257):
            run = run + 1 if steep[z, x] else 0
            best_z = max(best_z, run)
    # run along x (along the embankment crest)
    best_x = 0
    for z in range(257):
        run = 0
        for x in range(257):
            run = run + 1 if steep[z, x] else 0
            best_x = max(best_x, run)
    assert best_z >= 8, best_z
    assert best_x >= 8, best_x


def test_flat_30m_block(m0_dir):
    _, s = _slope(m0_dir)
    flat = (s < 5.0).astype(np.int32)
    ii = np.pad(flat, ((1, 0), (1, 0))).cumsum(0).cumsum(1)
    k = 31  # 31 samples = 30 m
    sums = ii[k:, k:] - ii[:-k, k:] - ii[k:, :-k] + ii[:-k, :-k]
    assert (sums == k * k).any()
