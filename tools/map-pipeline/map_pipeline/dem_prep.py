"""GDAL-free helpers of the DEM stage (M1-031): no-data filling, datum offset, range rounding,
input hashing and the exit codes of the ``dem`` command."""
from __future__ import annotations

import hashlib
from pathlib import Path

import numpy as np

EXIT_MISSING_DEM = 3
EXIT_SOURCES_NOT_OK = 4
EXIT_TOO_MUCH_NODATA = 5
MAX_NODATA_FRACTION = 0.01
DEM_SUFFIXES = (".tif", ".tiff", ".vrt")


def find_inputs(incoming: Path) -> list[Path]:
    if not incoming.is_dir():
        return []
    return sorted(p for p in incoming.rglob("*") if p.suffix.lower() in DEM_SUFFIXES and p.is_file())


def input_manifest(files: list[Path]) -> list[dict]:
    out = []
    for f in files:
        h = hashlib.sha256()
        with f.open("rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 20), b""):
                h.update(chunk)
        out.append({"file": f.name, "sha256": h.hexdigest()})
    return sorted(out, key=lambda e: e["file"])


def fill_nodata(h: np.ndarray, nodata: np.ndarray, max_fraction: float = MAX_NODATA_FRACTION):
    """Fills masked cells by repeatedly averaging valid 4-neighbours. Returns ``(filled, count)``;
    raises ``ValueError`` when more than ``max_fraction`` of the cells are masked."""
    count = int(nodata.sum())
    if count == 0:
        return h, 0
    if count > max_fraction * h.size:
        raise ValueError(f"{count} of {h.size} cells are no-data ({count / h.size:.2%} > {max_fraction:.0%})")
    out = h.astype(np.float64).copy()
    todo = nodata.copy()
    out[todo] = 0.0
    while todo.any():
        valid = ~todo
        acc = np.zeros_like(out)
        cnt = np.zeros_like(out)
        for ax, sh in ((0, 1), (0, -1), (1, 1), (1, -1)):
            v = np.roll(valid, sh, ax)
            val = np.roll(np.where(valid, out, 0.0), sh, ax)
            # np.roll wraps around: invalidate the wrapped edge
            if ax == 0:
                v[0 if sh == 1 else -1, :] = False
            else:
                v[:, 0 if sh == 1 else -1] = False
            acc += np.where(v, val, 0.0)
            cnt += v
        grow = todo & (cnt > 0)
        if not grow.any():
            raise ValueError("no-data region has no valid neighbour (empty raster?)")
        out[grow] = acc[grow] / cnt[grow]
        todo &= ~grow
    return out, count


def round_range(lo: float, hi: float, step: float = 0.1) -> tuple[float, float]:
    """Outward rounding to ``step`` metres (MAPS §1.2)."""
    hmin = float(np.floor(round(lo / step, 9)) * step)
    hmax = float(np.ceil(round(hi / step, 9)) * step)
    if hmax <= hmin:
        hmax = hmin + step
    return round(hmin, 6), round(hmax, 6)


def datum_offset(heights: np.ndarray, water: np.ndarray | None = None, fallback_percentile: float = 0.1) -> float:
    """Height of the lowest water surface (so water sits at ~0 m). Without a water mask the
    ``fallback_percentile`` of all heights is used instead of the raw minimum (robust to spikes)."""
    if water is not None and water.any():
        return float(np.min(heights[water]))
    return float(np.percentile(heights, fallback_percentile))


def water_mask_from_features(water_features: list[dict], shape: tuple[int, int], res: float = 1.0) -> np.ndarray:
    """Rasterise ``water.geojson`` features onto a boolean mask matching ``heights.shape`` (GDAL-free).
    Row 0 = south (y = 0), col 0 = west (x = 0). Uses open-water polygons (excluding flood_marsh) and
    river/canal centrelines buffered by half-width."""
    from . import apply_edits

    mask = np.zeros(shape, dtype=bool)
    nrows, ncols = shape
    for f in water_features:
        p, g = f.get("properties", {}), f.get("geometry", {})
        kind = p.get("kind")
        gtype = g.get("type")
        if gtype == "Polygon" and kind != "flood_marsh":
            ring = [tuple(q) for q in g["coordinates"][0][:-1]]
            if len(ring) < 3:
                continue
            xs, ys = [q[0] for q in ring], [q[1] for q in ring]
            c0, c1 = max(0, int(np.floor(min(xs) / res))), min(ncols - 1, int(np.ceil(max(xs) / res)))
            r0, r1 = max(0, int(np.floor(min(ys) / res))), min(nrows - 1, int(np.ceil(max(ys) / res)))
            if c1 < c0 or r1 < r0:
                continue
            X, Y = np.meshgrid(np.arange(c0, c1 + 1) * res, np.arange(r0, r1 + 1) * res)
            mask[r0 : r1 + 1, c0 : c1 + 1] |= apply_edits._inside(X, Y, ring)
        elif gtype == "LineString" and kind in ("river", "canal"):
            pts = g["coordinates"]
            half = float(p.get("width_m", 10.0)) / 2.0
            for a, b in zip(pts, pts[1:]):
                c0 = max(0, int(np.floor((min(a[0], b[0]) - half) / res)))
                c1 = min(ncols - 1, int(np.ceil((max(a[0], b[0]) + half) / res)))
                r0 = max(0, int(np.floor((min(a[1], b[1]) - half) / res)))
                r1 = min(nrows - 1, int(np.ceil((max(a[1], b[1]) + half) / res)))
                if c1 < c0 or r1 < r0:
                    continue
                X, Y = np.meshgrid(np.arange(c0, c1 + 1) * res, np.arange(r0, r1 + 1) * res)
                dx, dy = b[0] - a[0], b[1] - a[1]
                L = dx * dx + dy * dy
                t = np.clip(((X - a[0]) * dx + (Y - a[1]) * dy) / L, 0.0, 1.0) if L else np.zeros_like(X)
                d = np.hypot(X - a[0] - t * dx, Y - a[1] - t * dy)
                mask[r0 : r1 + 1, c0 : c1 + 1] |= d <= half
    return mask
