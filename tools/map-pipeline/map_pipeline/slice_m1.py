"""M1 slice (MAPS §3.8): full 1 m detail inside a rectangle, the 8 m far ring (bilinear) outside,
a linear feather band between them (M1-031). Pure numpy; GDAL is not needed here.

Local frame: x east, y north, origin SW; arrays are ``[row=y, col=x]`` (south-to-north rows).
The 2049x2049 near grid has sample k at local x = k m. The far ring (1025x1025, 8 m) is centred on
the playable area, so far sample i sits at x = i*8 - 3072.
"""
from __future__ import annotations

import numpy as np

NEAR_N = 2049
FAR_N = 1025
FAR_RES = 8.0
FAR_ORIGIN_M = 1024.0 - 4096.0  # local x/y of far sample 0
RECT_DESIGN = (380.0, 520.0, 1560.0, 1700.0)  # x0, y0, x1, y1
FEATHER_M = 32.0
BLOCKER_INSET_M = 4.0


def far_to_near_grid(far: np.ndarray, n: int = NEAR_N) -> np.ndarray:
    """Bilinear upsample of the far ring onto the 1 m near grid."""
    g = (np.arange(n, dtype=np.float64) - FAR_ORIGIN_M) / FAR_RES
    i0 = np.clip(np.floor(g).astype(int), 0, far.shape[0] - 2)
    t = np.clip(g - i0, 0.0, 1.0)
    rows = far[i0, :] * (1 - t)[:, None] + far[i0 + 1, :] * t[:, None]  # (n, FAR_N)
    return rows[:, i0] * (1 - t)[None, :] + rows[:, i0 + 1] * t[None, :]


def mask(n: int = NEAR_N, rect=RECT_DESIGN, feather_m: float = FEATHER_M) -> np.ndarray:
    """Weight of the 1 m data: 1 inside ``rect``, 0 at ``feather_m`` outside it, linear between
    (Chebyshev distance to the rectangle)."""
    x0, y0, x1, y1 = rect
    xs = np.arange(n, dtype=np.float64)
    dx = np.maximum(np.maximum(x0 - xs, xs - x1), 0.0)
    dy = np.maximum(np.maximum(y0 - xs, xs - y1), 0.0)
    d = np.maximum(dx[None, :], dy[:, None])
    return np.clip(1.0 - d / feather_m, 0.0, 1.0)


def blend(near: np.ndarray, far: np.ndarray, rect=RECT_DESIGN, feather_m: float = FEATHER_M):
    """Returns ``(heights, weight)``: ``near`` where weight == 1, upsampled ``far`` where 0."""
    if near.shape != (NEAR_N, NEAR_N):
        raise ValueError(f"near must be {NEAR_N}x{NEAR_N}, got {near.shape}")
    w = mask(near.shape[0], rect, feather_m)
    return w * near + (1.0 - w) * far_to_near_grid(far, near.shape[0]), w


def blocker_polygon(rect=RECT_DESIGN, inset_m: float = BLOCKER_INSET_M) -> dict:
    """Wall band for the sim: the rectangle outline (0 m outward) and the same rectangle inset by
    ``inset_m``. Rings are counter-clockwise, outer first."""
    x0, y0, x1, y1 = rect
    ring = lambda a, b, c, d: [[a, b], [c, b], [c, d], [a, d]]  # noqa: E731
    return {
        "outer": ring(x0, y0, x1, y1),
        "inner": ring(x0 + inset_m, y0 + inset_m, x1 - inset_m, y1 - inset_m),
        "width_m": inset_m,
    }


def meta_block(rect=RECT_DESIGN, feather_m: float = FEATHER_M) -> dict:
    return {
        "rect_design": list(rect),
        "rect_local": list(rect),  # the frame is the local system until the similarity is applied
        "feather_m": feather_m,
        "blocker_polygon": blocker_polygon(rect),
    }
