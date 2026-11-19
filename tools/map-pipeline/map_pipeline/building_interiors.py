"""Asset-free interiors: bake one shared geometry document for render and sim.

Build after `zones` annotates doors. Coordinates are local metres (x, height, z).
Walls are oriented boxes; floors, roofs and ramps are indexed surfaces (upward normals).
"""
from __future__ import annotations

import json
import math
from pathlib import Path

from . import clean, export

LEVEL_M = 3.0
WALL_M = 0.30
DOOR_HEIGHT_M = 2.1
RAMP_RUN_M = 5.0  # arctan(3/5) ≈ 31°, below the 35° KCC limit
RAMP_WIDTH_M = 1.5
EPS = 1e-8


def add(a, b, t=1.0):
    return (a[0] + b[0] * t, a[1] + b[1] * t)


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b, p):
    return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])


def edges(ring):
    return zip(ring, ring[1:] + ring[:1])


def contains(ring, p):
    x, z = p
    return sum((az > z) != (bz > z) and x < ax + (bx - ax) * (z - az) / (bz - az)
               for (ax, az), (bx, bz) in edges(ring)) % 2 == 1


def triangles(ring):
    """Ear-clip a simple polygon; throw instead of silently filling an invalid footprint."""
    pts = list(ring if clean.area(ring) > 0 else ring[::-1])
    out = []
    while len(pts) > 3:
        for i, b in enumerate(pts):
            a, c = pts[i - 1], pts[(i + 1) % len(pts)]
            turn = cross(a, b, c)
            if abs(turn) <= EPS:
                del pts[i]
                break
            if turn > 0 and not any(all(cross(u, v, p) >= -EPS for u, v in ((a, b), (b, c), (c, a)))
                                    for p in pts if p not in (a, b, c)):
                out.append([a, b, c])
                del pts[i]
                break
        else:
            raise ValueError('untriangulatable footprint')
    if clean.area(pts) > EPS:
        out.append(pts)
    if not out:
        raise ValueError('empty footprint')
    return out


def halfplane(poly, a, b, sign):
    if not poly:
        return []
    out = []
    for p, q in edges(poly):
        d, e = cross(a, b, p) * sign, cross(a, b, q) * sign
        if abs(d) < EPS:
            d = 0
        if abs(e) < EPS:
            e = 0
        if d >= 0:
            out.append(p)
        if (d < 0 < e) or (e < 0 < d):
            out.append(add(p, sub(q, p), d / (d - e)))
    return out


def cut(polys, hole):
    """Subtract a convex CCW hole: emit the outside piece at each half-plane once."""
    out = []
    for poly in polys:
        clipped = poly
        for a, b in edges(hole):
            clipped = halfplane(clipped, a, b, 1)
        if len(clipped) < 3 or clean.area(clipped) <= EPS:
            out.append(poly)
            continue
        for a, b in edges(hole):
            outside = halfplane(poly, a, b, -1)
            if len(outside) >= 3 and clean.area(outside) > EPS:
                out.append(outside)
            poly = halfplane(poly, a, b, 1)
    return out


def surface(kind, polys, height):
    """Weld shared points (millimetre x/z, finer y); upward orientation in 3D."""
    verts, indices, known = [], [], {}
    for poly in polys:
        for i in range(1, len(poly) - 1):
            tri = [(round(x, 4), round(height((x, z)), 4), round(z, 4)) for x, z in (poly[0], poly[i + 1], poly[i])]
            if abs(cross(tri[0][::2], tri[1][::2], tri[2][::2])) < 1e-6:
                continue
            for v in tri:
                if v not in known:
                    known[v] = len(verts)
                    verts.append(v)
                indices.append(known[v])
    return {'kind': kind, 'vertices': verts, 'indices': indices}


def plan_ramp(rings, door_inside, floor_polys, ground, base):
    """Find a straight corridor with a metre of landing at both ends and side clearance.

    Test the entire corridor against the polygon (including courtyards), not just its corners.
    """
    ring = rings[0]
    xmin, zmin = map(min, zip(*ring))
    xmax, zmax = map(max, zip(*ring))
    grid = [(x / 2, z / 2) for x in range(math.ceil(xmin * 2), math.ceil(xmax * 2), 2)
            for z in range(math.ceil(zmin * 2), math.ceil(zmax * 2), 2)]
    centers = [(sum(x for x, _ in p) / len(p), sum(z for _, z in p) / len(p)) for p in floor_polys]
    angles = [math.atan2(b[1] - a[1], b[0] - a[0]) for a, b in edges(ring)]
    angles = sorted(set(round((a + delta) % math.pi, 5) for a in angles + [0] for delta in (0, math.pi / 2)))
    # Search near the door first; do not place the ramp across the entry corridor.
    for center in sorted(grid + centers, key=lambda p: math.dist(p, door_inside)):
        if not contains(ring, center):
            continue
        for angle in angles:
            axis = (math.cos(angle), math.sin(angle))
            if dot(sub(center, door_inside), axis) < 0:
                axis = (-axis[0], -axis[1])
            side = (-axis[1], axis[0])
            start = add(center, axis, -RAMP_RUN_M / 2)
            end = add(start, axis, RAMP_RUN_M)
            w = RAMP_WIDTH_M / 2 + 0.65
            corridor = [add(add(start, axis, d), side, q) for d, q in
                        ((-1, -w), (RAMP_RUN_M + 1, -w), (RAMP_RUN_M + 1, w), (-1, w))]
            if not all(contains(ring, p) and not any(contains(h, p) for h in rings[1:]) for p in corridor):
                continue
            if abs(dot(sub(door_inside, center), axis)) < RAMP_RUN_M / 2 + 0.5 and abs(dot(sub(door_inside, center), side)) < w:
                continue
            if any(contains(corridor, a) or any(clean._seg_x(a, b, p, q) for p, q in edges(corridor))
                   for r in rings for a, b in edges(r)):
                continue
            if any(ground(*add(add(start, axis, d), side, q)) > base + d * LEVEL_M / RAMP_RUN_M + 0.06
                   for d in (0, 0.5, 1, 2, 3, 4, 5) for q in (-0.6, 0, 0.6)):
                continue
            return start, end, axis, side
    return None


def bake_building(feature, ground):
    props = feature['properties']
    rings = [clean.dedupe_points(r, closed=True) for r in feature['geometry']['coordinates']]
    ring = rings[0]
    door = props['door']
    width = float(door['width_m'])
    candidates = []
    for i, (a, b) in enumerate(edges(ring)):
        length = math.dist(a, b)
        axis = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
        t = dot(sub(door['center'], a), axis)
        if width >= 1 and width / 2 + WALL_M < t < length - width / 2 - WALL_M:
            center = add(a, axis, t)
            candidates.append((math.dist(center, door['center']), i, a, b, center, axis))
    if not candidates or min(candidates)[0] > 0.02:
        raise ValueError('invalid authored doorway')
    _, door_edge, a, b, center, axis = min(candidates)
    outward = (axis[1], -axis[0]) if clean.area(ring) > 0 else (-axis[1], axis[0])
    inside = add(center, outward, -0.9)
    # Keep the threshold in step range, with a single continuous floor at the doorway.
    jambs = [add(center, axis, d) for d in (-width / 2, width / 2)]
    base = max(ground(*add(p, outward, t)) for p in [center, *jambs] for t in (-0.3, 0, 0.3)) + 0.02
    base = round(base, 4)
    bottom = min(ground(*p) for r in rings for p in r) - 0.4
    levels = int(props['levels'])
    roof = base + levels * LEVEL_M
    walls = []

    def wall(a, b, lo, hi):
        walls.append(dict(a=a, b=b, bottom=lo, top=hi, width=WALL_M))

    for h, r in enumerate(rings):
        for i, (a, b) in enumerate(edges(r)):
            if h == 0 and i == door_edge:
                left, right = jambs
                wall(a, left, bottom, roof)
                wall(right, b, bottom, roof)
                wall(left, right, base + DOOR_HEIGHT_M, roof)
            else:
                wall(a, b, bottom, roof)
    floor = triangles(ring)
    for hole in rings[1:]:
        for tri in triangles(hole):
            floor = cut(floor, tri)
    surfaces = [surface('floor', floor, lambda _: base), surface('roof', floor, lambda _: roof)]
    ramp = plan_ramp(rings, inside, floor, ground, base) if levels > 1 else None
    playable = levels if ramp else 1
    if ramp:
        start, end, axis, side = ramp
        w = RAMP_WIDTH_M / 2
        strip = [add(start, side, -w), add(end, side, -w), add(end, side, w), add(start, side, w)]
        # The floor above the ramp starts with a 2.1 m clear head opening.
        front = add(start, axis, RAMP_RUN_M * (1 - 2.2 / LEVEL_M))
        hole = [add(front, side, -w), add(end, side, -w), add(end, side, w), add(front, side, w)]
        upper = cut(floor, hole)
        for i in range(1, playable):
            y = base + i * LEVEL_M
            surfaces.append(surface('floor', upper, lambda _: y))
            surfaces.append(surface('ramp', [strip], lambda p: y - LEVEL_M + dot(sub(p, start), axis) * LEVEL_M / RAMP_RUN_M))
    # Select an interior point on a floor polygon (centroids of concave outlines may lie outside).
    probes = []
    for i in range(playable):
        best = max(upper if i else floor, key=clean.area) if ramp else max(floor, key=clean.area)
        px, pz = (sum(v) / len(best) for v in zip(*best))
        probes.append([px, base + i * LEVEL_M + 1.2, pz])
    result = dict(id=props['_osm_id'], ring=ring, holes=rings[1:], base_y=base, roof_y=roof, walls=walls,
                  surfaces=surfaces, probes=probes, door=dict(center=center, outward=outward, width=width))
    if ramp:
        result['ramp'] = dict(start=[start[0], base, start[1]], end=[end[0], base + LEVEL_M, end[1]])
    return result


def bake(directory: Path):
    meta = json.loads((directory / 'meta.json').read_text())['height']
    q = export.read_r16(directory / 'heightmap_1m.r16', meta['samples'])
    h = export.dequantize(q, meta['hmin_m'], meta['hmax_m'])

    def ground(x, z):
        ix, iz = math.floor(x), math.floor(z)
        ix, iz = min(len(h) - 2, ix), min(len(h) - 2, iz)
        fx, fz = x - ix, z - iz
        return float((h[iz, ix] * (1 - fx) + h[iz, ix + 1] * fx) * (1 - fz) +
                     (h[iz + 1, ix] * (1 - fx) + h[iz + 1, ix + 1] * fx) * fz)

    features = json.loads((directory / 'buildings.geojson').read_text())['features']
    buildings = []
    for f in features:
        if f['properties'].get('enterable'):
            try:
                buildings.append(bake_building(f, ground))
            except ValueError as exc:
                raise ValueError(f"{f['properties']['_osm_id']}: {exc}") from exc
    buildings.sort(key=lambda b: b['id'])
    out = directory / 'building-interiors.json'
    # One building per line keeps rebuilds reviewable; six decimals exceed f32 precision at map scale.
    def rounded(value):
        if isinstance(value, float):
            return round(value, 6)
        if isinstance(value, (tuple, list)):
            return [rounded(v) for v in value]
        if isinstance(value, dict):
            return {k: rounded(v) for k, v in value.items()}
        return value

    out.write_text('{"version":1,"buildings":[\n' + ',\n'.join(json.dumps(rounded(b), separators=(',', ':')) for b in buildings) + '\n]}\n')
    return {'interiors': len(buildings), 'with_ramps': sum('ramp' in b for b in buildings), 'probes': sum(len(b['probes']) for b in buildings)}
