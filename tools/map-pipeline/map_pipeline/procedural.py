"""Deterministic procedural terrain for M0 (no external data).

Coordinates: x east, z north, metres, SW corner of the playable block = (0, 0).
Arrays are row-major with row index = z and column index = x.

h(x, z) is an analytic function of world position, so the 1 m near block and the 8 m far
ring are samples of the *same* surface (the far ring centre matches the near block exactly).
"""
from __future__ import annotations

from dataclasses import asdict, dataclass, field

import numpy as np

NEAR_SAMPLES = 257  # 256 m at 1 m
FAR_SAMPLES = 1025  # 8192 m at 8 m
FAR_RES_M = 8.0
FAR_EXTENT_M = 8192.0
FAR_ORIGIN_IDX = (FAR_SAMPLES - 1) // 2 - 16  # far index of near-block x = 0 (4096 - 128 m) = 496
LATTICE = 256  # wrap size of every octave's random table


@dataclass(frozen=True)
class Params:
    wavelengths_m: tuple = (2048.0, 1024.0, 512.0, 256.0, 128.0, 64.0, 32.0, 16.0, 8.0)
    gain_mult: float = 0.5
    base_level_m: float = 2.2
    base_amp_m: float = 1.8
    # embankment: linear ramp (constant slope) rising along +z, between z0 and z0+width
    ramp_z0_m: float = 123.0
    ramp_width_m: float = 10.0
    ramp_height_m: float = 8.0  # slope = 0.8 -> 38.7 deg
    ramp_x_fade_m: tuple = (16.0, 40.0, 216.0, 240.0)  # rise start/end, fall start/end
    plateau_z_fade_m: tuple = (400.0, 800.0)  # plateau fades out far beyond the block
    # flat pads: centre x, z, half size of the exactly-flat core, falloff width, level
    pads: tuple = (
        ("us", 128.0, 56.0, 32.0, 24.0, 2.2),
        ("ger", 128.0, 200.0, 32.0, 24.0, 10.2),
    )

    def to_json(self) -> dict:
        d = asdict(self)
        d["pads"] = [
            dict(zip(("id", "cx_m", "cz_m", "half_m", "falloff_m", "level_m"), p)) for p in self.pads
        ]
        return json_ready(d)


def json_ready(o):
    if isinstance(o, tuple):
        return [json_ready(v) for v in o]
    if isinstance(o, list):
        return [json_ready(v) for v in o]
    if isinstance(o, dict):
        return {k: json_ready(v) for k, v in o.items()}
    return o


def _smoothstep(x, a, b):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


class Terrain:
    def __init__(self, seed: int, params: Params | None = None):
        self.seed = int(seed)
        self.p = params or Params()
        rng = np.random.Generator(np.random.PCG64(self.seed))
        n = len(self.p.wavelengths_m)
        self.tables = [rng.uniform(-1.0, 1.0, size=(LATTICE, LATTICE)) for _ in range(n)]
        self.shifts = [(int(rng.integers(0, LATTICE)), int(rng.integers(0, LATTICE))) for _ in range(n)]
        w = np.array([self.p.gain_mult**k for k in range(n)])
        self.weights = w / w.sum()

    def _fbm(self, xs: np.ndarray, zs: np.ndarray) -> np.ndarray:
        out = np.zeros((zs.size, xs.size))
        for tab, (sx, sz), wt, lam in zip(
            self.tables, self.shifts, self.weights, self.p.wavelengths_m
        ):
            ux = xs / lam
            uz = zs / lam
            ix = np.floor(ux)
            iz = np.floor(uz)
            fx = ux - ix
            fz = uz - iz
            sxm = fx * fx * (3.0 - 2.0 * fx)
            szm = fz * fz * (3.0 - 2.0 * fz)
            ix0 = (ix.astype(np.int64) + sx) % LATTICE
            iz0 = (iz.astype(np.int64) + sz) % LATTICE
            ix1 = (ix0 + 1) % LATTICE
            iz1 = (iz0 + 1) % LATTICE
            a = tab[iz0[:, None], ix0[None, :]]
            b = tab[iz0[:, None], ix1[None, :]]
            c = tab[iz1[:, None], ix0[None, :]]
            d = tab[iz1[:, None], ix1[None, :]]
            top = a + (b - a) * sxm[None, :]
            bot = c + (d - c) * sxm[None, :]
            out += wt * (top + (bot - top) * szm[:, None])
        return out

    def height(self, xs: np.ndarray, zs: np.ndarray) -> np.ndarray:
        """Height in metres on the grid xs (columns) x zs (rows)."""
        xs = np.asarray(xs, dtype=np.float64)
        zs = np.asarray(zs, dtype=np.float64)
        p = self.p
        h = p.base_level_m + p.base_amp_m * self._fbm(xs, zs)
        # embankment ("tu kan"): constant-slope ramp, masked in x and faded far in +z
        a, b, c, d = p.ramp_x_fade_m
        xmask = _smoothstep(xs, a, b) * (1.0 - _smoothstep(xs, c, d))
        ramp = np.clip((zs - p.ramp_z0_m) / p.ramp_width_m, 0.0, 1.0)
        zfade = 1.0 - _smoothstep(zs, *p.plateau_z_fade_m)
        h = h + p.ramp_height_m * (ramp * zfade)[:, None] * xmask[None, :]
        # flat pads (exactly flat core, smooth falloff)
        for _id, cx, cz, half, fall, level in p.pads:
            dx = np.maximum(np.abs(xs - cx) - half, 0.0)
            dz = np.maximum(np.abs(zs - cz) - half, 0.0)
            dist = np.sqrt(dx[None, :] ** 2 + dz[:, None] ** 2)
            m = 1.0 - _smoothstep(dist, 0.0, fall)
            h = h + (level - h) * m
        return h

    def near(self) -> np.ndarray:
        g = np.arange(NEAR_SAMPLES, dtype=np.float64)
        return self.height(g, g)

    def far(self) -> np.ndarray:
        g = (np.arange(FAR_SAMPLES) - FAR_ORIGIN_IDX) * FAR_RES_M
        return self.height(g, g)

    def params_json(self) -> dict:
        return self.p.to_json()
