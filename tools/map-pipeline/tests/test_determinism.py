import hashlib
from pathlib import Path

from map_pipeline.cli import generate_m0


def _hashes(d: Path):
    return {str(p.relative_to(d)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(d.rglob("*")) if p.is_file()}


def test_same_seed_same_bytes(tmp_path):
    generate_m0(tmp_path / "a", seed=1)
    generate_m0(tmp_path / "b", seed=1)
    ha, hb = _hashes(tmp_path / "a"), _hashes(tmp_path / "b")
    assert len(ha) == 2 + 32 + 3  # 2 heightmaps, 32 chunks, meta/spawns/manifest
    assert ha == hb


def test_different_seed_differs(tmp_path):
    generate_m0(tmp_path / "a", seed=1)
    generate_m0(tmp_path / "b", seed=2)
    assert _hashes(tmp_path / "a")["heightmap_1m.r16"] != _hashes(tmp_path / "b")["heightmap_1m.r16"]
