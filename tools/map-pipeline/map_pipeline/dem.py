"""DEM path (MAPS 1.2): gdalwarp -r average to 1 m, crop a 256 m window. GDAL imported lazily."""
from __future__ import annotations

import numpy as np

from .procedural import FAR_ORIGIN_IDX, FAR_RES_M, FAR_SAMPLES, NEAR_SAMPLES


class GdalMissing(RuntimeError):
    pass


def require_gdal():
    try:
        from osgeo import gdal  # noqa: PLC0415
    except ImportError as e:
        raise GdalMissing(
            "The DEM path needs the GDAL python bindings (`osgeo`), which are not installed. "
            "Install GDAL (e.g. apt install python3-gdal) or omit --dem to use the procedural terrain."
        ) from e
    gdal.UseExceptions()
    return gdal


def window_bounds(e: float, n: float, res_m: float = 1.0, samples: int = NEAR_SAMPLES):
    """Warp output bounds so that pixel *centres* are the sample points e + i*res, n + j*res.

    (E, N) is the south-west sample point of the window. Returns (xmin, ymin, xmax, ymax).
    """
    half = res_m / 2.0
    span = (samples - 1) * res_m
    return (e - half, n - half, e + span + half, n + span + half)


def _warp(gdal, src: str, bounds, res_m: float) -> np.ndarray:
    xmin, ymin, xmax, ymax = bounds
    w = int(round((xmax - xmin) / res_m))
    h = int(round((ymax - ymin) / res_m))
    ds = gdal.Warp(
        "",
        src,
        format="MEM",
        outputBounds=(xmin, ymin, xmax, ymax),
        width=w,
        height=h,
        resampleAlg="average",
        dstNodata=-9999,
        outputType=gdal.GDT_Float32,
    )
    arr = ds.GetRasterBand(1).ReadAsArray().astype(np.float64)
    ds = None
    arr = np.flipud(arr)  # GDAL rows run north->south; ours run south->north
    return arr


def load_dem(path: str, e: float, n: float):
    """Return (near 257x257, far 1025x1025) heights in metres (south-up row order)."""
    gdal = require_gdal()
    near = _warp(gdal, path, window_bounds(e, n, 1.0, NEAR_SAMPLES), 1.0)
    if (near <= -9998).any():
        raise ValueError("DEM does not fully cover the requested 256 m window")
    # far ring: 8 m average around the block; origin of far sample 0 = block SW - FAR_ORIGIN_IDX*8
    fe = e - FAR_ORIGIN_IDX * FAR_RES_M
    fn = n - FAR_ORIGIN_IDX * FAR_RES_M
    far = _warp(gdal, path, window_bounds(fe, fn, FAR_RES_M, FAR_SAMPLES), FAR_RES_M)
    nodata = far <= -9998
    if nodata.any():
        far[nodata] = near.min()  # outside DEM coverage: fill with the block minimum
    # keep the far ring continuous with the near block at the 8 m sample points
    lo = FAR_ORIGIN_IDX
    far[lo : lo + 33, lo : lo + 33] = near[::8, ::8]
    return near, far
