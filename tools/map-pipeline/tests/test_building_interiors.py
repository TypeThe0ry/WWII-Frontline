import json
from pathlib import Path

import pytest

from map_pipeline.building_interiors import bake, bake_building, contains, cut, triangles
from map_pipeline.clean import area


def test_polygon_cut_preserves_concavity_and_courtyard_area():
    outer = [(0, 0), (8, 0), (8, 8), (4, 8), (4, 4), (0, 4)]
    hole = [(5, 2), (7, 2), (7, 4), (5, 4)]
    pieces = cut(triangles(outer), hole)
    assert sum(area(p) for p in pieces) == pytest.approx(area(outer) - area(hole))
    assert not any(contains(p, (6, 3)) for p in pieces)


def test_authored_opening_and_shared_stairwell():
    building = bake_building({
        'properties': {'_osm_id': 'fixture', 'levels': 2, 'door': {'center': [5, 0], 'width_m': 1.2}},
        'geometry': {'coordinates': [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]},
    }, lambda _x, _z: 0)
    door = building['door']
    assert door['outward'] == (0, -1)
    lintel = next(w for w in building['walls'] if w['bottom'] > building['base_y'])
    assert lintel['bottom'] - building['base_y'] >= 2
    assert lintel['a'][0] == pytest.approx(4.4)
    assert lintel['b'][0] == pytest.approx(5.6)
    assert len(building['probes']) == 2
    start, end = building['ramp']['start'], building['ramp']['end']
    assert end[1] - start[1] == pytest.approx(3)
    assert sum(s['kind'] == 'ramp' for s in building['surfaces']) == 1
    assert all(s['indices'] for s in building['surfaces'])


def test_checked_in_carentan_layout_matches_bake(tmp_path):
    directory = Path(__file__).resolve().parents[3] / 'data/maps/carentan'
    for name in ('meta.json', 'heightmap_1m.r16', 'buildings.geojson'):
        (tmp_path / name).symlink_to(directory / name)
    summary = bake(tmp_path)
    out = tmp_path / 'building-interiors.json'
    assert out.read_bytes() == (directory / out.name).read_bytes()
    layouts = json.loads(out.read_text())['buildings']
    assert summary == {'interiors': 70, 'with_ramps': 54, 'probes': 146}
    assert len({b['id'] for b in layouts}) == len(layouts)
    for b in layouts:
        for x, y, z in b['probes']:
            assert b['base_y'] < y < b['roof_y']
            assert contains(b['ring'], (x, z)) and not any(contains(h, (x, z)) for h in b['holes'])
