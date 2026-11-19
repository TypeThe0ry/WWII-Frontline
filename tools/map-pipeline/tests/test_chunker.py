import json
from pathlib import Path

import numpy as np
import pytest

from map_pipeline import chunker as cb, splat, vegetation

ROOT = Path(__file__).resolve().parents[3]
DATA = ROOT / "data/maps/carentan"


def test_delta_roundtrip():
    rng = np.random.default_rng(1)
    q = rng.integers(0, 65536, (65, 65), dtype=np.uint16)
    assert np.array_equal(cb.delta_decode(cb.delta_encode(q)), q)
    q2 = np.cumsum(rng.integers(0, 4, (65, 65)), axis=1).astype(np.uint16)
    assert np.array_equal(cb.delta_decode(cb.delta_encode(q2)), q2)


def test_container_roundtrip():
    c = cb.pack_container(7, 9, {1: (b"abc", 3), 5: (b"zz", 9)})
    u = cb.unpack_container(c)
    assert (u["bx"], u["bz"]) == (7, 9) and u["sections"][5] == (b"zz", 9)
    with pytest.raises(ValueError):
        cb.unpack_container(b"XXXX" + c[4:])


def _layers(**kw):
    base = {"buildings": [], "roads": [], "water": [], "hedgerows": [], "forests": []}
    base.update(kw)
    return base


def _splat(layers, h=None, bx=3, bz=3):
    hw = np.zeros((72, 72)) if h is None else h
    return splat.splat_chunk(bx, bz, layers, hw)


def poly(x0, y0, x1, y1, **props):
    ring = [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
    return {"geometry": {"type": "Polygon", "coordinates": [ring]}, "properties": props}


def line(pts, **props):
    return {"geometry": {"type": "LineString", "coordinates": pts}, "properties": props}


def test_splat_sum_and_base_grass():
    w, _, _ = _splat(_layers())
    assert (w.sum(axis=-1, dtype=int) == 255).all() and (w[..., splat.L["grass"]] == 255).all()


def test_splat_rules_each():
    cx = 3 * 64 + 32.0
    # slope > 35 deg -> rock
    h = np.tile(np.arange(72) * 1.0, (72, 1))  # 45 deg
    w, _, _ = _splat(_layers(), h)
    assert w[32, 32, splat.L["rock"]] > 200
    # paved road -> road (no urban)
    road = line([[cx - 30, cx], [cx + 30, cx]], **{"class": "paved", "width_m": 6.0, "bridge": False})
    w, _, _ = _splat(_layers(roads=[road]))
    assert w[32, 32, splat.L["road"]] > 200
    # forest -> forest_floor
    w, _, _ = _splat(_layers(forests=[poly(cx - 20, cx - 20, cx + 20, cx + 20)]))
    assert w[32, 32, splat.L["forest_floor"]] > 200
    # building buffer -> dirt, 2 m ring
    w, _, _ = _splat(_layers(buildings=[poly(cx - 5, cx - 5, cx + 5, cx + 5)]))
    assert w[32, 32 + 6, splat.L["dirt"]] > 100 and w[32, 32 + 12, splat.L["grass"]] > 200
    # water edge 3 m -> sand (river), water -> mud
    riv = line([[cx - 30, cx], [cx + 30, cx]], kind="river", width_m=6.0)
    w, _, _ = _splat(_layers(water=[riv]))
    assert w[32, 32, splat.L["mud"]] > 200 and w[36, 32, splat.L["sand"]] > 100
    # flood marsh -> mud
    fm = poly(cx - 10, cx - 10, cx + 10, cx + 10, kind="flood_marsh")
    w, _, _ = _splat(_layers(water=[fm]))
    assert w[32, 32, splat.L["mud"]] > 200


def test_splat_urban_cobble():
    cx = 3 * 64 + 32.0
    blds = [poly(3 * 64 + 2 + 12 * i, 3 * 64 + 2, 3 * 64 + 12 + 12 * i, 3 * 64 + 12) for i in range(5)]
    road = line([[3 * 64, cx + 10], [4 * 64, cx + 10]], **{"class": "paved", "width_m": 6.0, "bridge": False})
    w, _, _ = _splat(_layers(buildings=blds, roads=[road]))
    assert w[42, 32, splat.L["cobble"]] > 200


def test_vegetation_determinism_and_exclusion():
    cx = 3 * 64 + 32.0
    layers = _layers(forests=[poly(3 * 64, 3 * 64, 4 * 64, 4 * 64)],
                     roads=[line([[3 * 64, cx], [4 * 64, cx]], **{"class": "paved", "width_m": 6.0, "bridge": False})])
    _, m, _ = _splat(layers)
    a = vegetation.place("t", 3, 3, m, layers["forests"], [], 1.0)
    b = vegetation.place("t", 3, 3, m, layers["forests"], [], 1.0)
    assert a == b and len(a[0]) > 20
    for x, z, *_ in a[0]:
        assert abs(z / 65535 * 64 - 32.0) > 2.5  # outside the road
    assert vegetation.seed_for("t", 3, 3) != vegetation.seed_for("t", 3, 4)


def test_tri_height_matches_corners_and_diagonal():
    h = np.zeros((65, 65))
    h[1, 1] = 2.0  # raise the diagonal vertex
    assert cb.tri_height(h, 1.0, 1.0) == pytest.approx(2.0)
    assert cb.tri_height(h, 0.5, 0.5) == pytest.approx(1.0)  # on the shared diagonal


@pytest.mark.skipif(not (DATA / "heightmap_1m.r16").exists(), reason="needs Carentan data")
def test_bake_subset_edges_sizes_and_determinism(tmp_path):
    ks = [(14, 14), (15, 14), (14, 15), (18, 14)]
    ctx, res, ent = cb.bake(DATA, tmp_path / "a", workers=1, subset=ks)
    _, _, ent2 = cb.bake(DATA, tmp_path / "b", workers=1, subset=ks)
    assert [e["sha256"] for e in ent] == [e["sha256"] for e in ent2]
    assert all(e["bytes"] <= cb.MAX_CHUNK_BYTES for e in ent)
    v = cb.verify(ctx, res, tmp_path / "a")
    assert v["edge_mismatches"] == 0 and v["splat_bad_pixels"] == 0 and v["raycast_max_dh_m"] <= 0.05


def test_real_report():
    r = json.loads((ROOT / "reports/m1-chunk-bake.json").read_text())
    assert r["chunks"] == 380 and not r["chunks_over_limit"] and r["bytes"]["max"] <= cb.MAX_CHUNK_BYTES
    assert r["verify"]["edge_mismatches"] == 0 and r["verify"]["veg_violations"] == 0 and r["verify"]["raycast_max_dh_m"] <= 0.05
    assert r["fpak"]["chunks_packed"] == 380 and r["fpak"]["assert_no_standalone"] and r["fpak"]["consistency_ok"]
    idx = json.loads((DATA / "m1/chunks/index.json").read_text())
    got = {(c["bx"], c["bz"]) for c in idx["chunks"]}
    assert got == set(cb.keys())
    m = json.loads((ROOT / "assets/manifests/terrain_carentan.json").read_text())
    assert len(m["assets"]) == 380 and m["schema"] == 1


def test_pack_chunks_fpak_roundtrip_and_determinism(tmp_path):
    from map_pipeline import pack_chunks as pc
    ks = [(14, 14), (15, 14), (14, 15), (18, 14)]
    cb.bake(DATA, tmp_path / "chunks", workers=1, subset=ks)
    s1 = pc.pack_chunks(tmp_path / "chunks", tmp_path / "fpak1", tmp_path / "m1.json")
    s2 = pc.pack_chunks(tmp_path / "chunks", tmp_path / "fpak2", tmp_path / "m2.json")
    assert s1["chunks_packed"] == 4 and s1["entries_packed"] == 8
    assert s1["manifest_sha256"] == s2["manifest_sha256"]
    assert s1["packs_sha256"] == s2["packs_sha256"]
    assert s1["assert_no_standalone"] and s1["consistency_ok"]


def test_vegetation_10k_synthetic_exclusion():
    # Dense synthetic forest + road/building/water exclusions across 120 chunks -> >= 10,000 placed instances, 0 violations
    checked = violations = 0
    for idx in range(120):
        bx, bz = 5 + (idx % 12), 8 + (idx // 12)
        x0, z0 = bx * 64.0, bz * 64.0
        layers = _layers(
            forests=[poly(x0, z0, x0 + 64, z0 + 64)],
            roads=[line([[x0, z0 + 32], [x0 + 64, z0 + 32]], **{"class": "paved", "width_m": 6.0, "bridge": False})],
            buildings=[poly(x0 + 10, z0 + 10, x0 + 20, z0 + 20)],
            water=[poly(x0 + 44, z0 + 44, x0 + 56, z0 + 56, kind="pond")],
        )
        _, m, _ = _splat(layers, bx=bx, bz=bz)
        trees, shrubs, _ = vegetation.place("synth", bx, bz, m, layers["forests"], [], 1.0)
        assert len(trees) <= vegetation.MAX_TREES and len(shrubs) <= vegetation.MAX_SHRUBS
        for xq, zq, *_ in trees + shrubs:
            checked += 1
            if not vegetation._allowed(m, bx, bz, x0 + xq / 65535 * 64, z0 + zq / 65535 * 64):
                violations += 1
    assert checked >= 10_000 and violations == 0
