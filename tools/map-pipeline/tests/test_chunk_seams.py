import numpy as np
import pytest

from map_pipeline import chunks as ck
from map_pipeline.export import read_r16


def _b(d, bx, bz):
    return np.frombuffer((d / "chunks" / f"b_{bx}_{bz}.bin").read_bytes(), dtype="<u2").reshape(65, 65)


def test_sizes(m0_dir):
    assert (m0_dir / "heightmap_1m.r16").stat().st_size == 257 * 257 * 2 == 132098
    assert (m0_dir / "heightmap_far_8m.r16").stat().st_size == 1025 * 1025 * 2 == 2101250
    for bz in range(4):
        for bx in range(4):
            assert (m0_dir / "chunks" / f"b_{bx}_{bz}.bin").stat().st_size == 8450
            assert (m0_dir / "chunks" / f"c_{bx}_{bz}.bin").stat().st_size == 16 + 8450 + 4096 == 12562


def test_seams_equal(m0_dir):
    for bz in range(4):
        for bx in range(4):
            if bx < 3:
                assert np.array_equal(_b(m0_dir, bx, bz)[:, 64], _b(m0_dir, bx + 1, bz)[:, 0])
            if bz < 3:
                assert np.array_equal(_b(m0_dir, bx, bz)[64, :], _b(m0_dir, bx, bz + 1)[0, :])


def test_chunks_match_heightmap(m0_dir):
    q = read_r16(m0_dir / "heightmap_1m.r16", 257)
    for bz in range(4):
        for bx in range(4):
            assert np.array_equal(_b(m0_dir, bx, bz), q[bz * 64 : bz * 64 + 65, bx * 64 : bx * 64 + 65])


def test_chunk_index_bounds():
    q = np.zeros((257, 257), dtype="<u2")
    with pytest.raises(IndexError):
        ck.chunk_slice(q, 4, 0)
    with pytest.raises(IndexError):
        ck.chunk_slice(q, 0, -1)


def test_collision_format(m0_dir):
    data = (m0_dir / "chunks" / "c_2_1.bin").read_bytes()
    assert data[:4] == b"FCOL" and data[4] == 1 and data[5] == 0
    assert int.from_bytes(data[6:8], "little") == 65
    hmin, hmax, h, mats = ck.decode_c(data)
    assert np.array_equal(h, _b(m0_dir, 2, 1))
    assert mats.shape == (64, 64) and not mats.any()


def test_far_ring_matches_near(m0_dir):
    near = read_r16(m0_dir / "heightmap_1m.r16", 257)
    far = read_r16(m0_dir / "heightmap_far_8m.r16", 1025)
    c = 496
    diff = np.abs(far[c : c + 33, c : c + 33].astype(int) - near[::8, ::8].astype(int))
    assert diff.max() <= 1
