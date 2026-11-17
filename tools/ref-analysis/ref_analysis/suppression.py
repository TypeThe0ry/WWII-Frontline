"""M-IMG (Laplacian variance) and M-FLOW (optical-flow shake) curves; vignette / saturation
(REFERENCE_MEASUREMENTS 3.7). Outputs numbers only, never images."""

from __future__ import annotations

from dataclasses import dataclass

import cv2
import numpy as np

from .extract_frames import Crop, iter_frames


def _center_roi(h: int, w: int, frac: float) -> tuple[slice, slice]:
    dh, dw = int(h * (1 - frac) / 2), int(w * (1 - frac) / 2)
    return slice(dh, h - dh), slice(dw, w - dw)


def laplacian_variance(gray: np.ndarray, roi_frac: float = 0.5, mask: np.ndarray | None = None) -> float:
    ys, xs = _center_roi(*gray.shape[:2], roi_frac)
    lap = cv2.Laplacian(gray[ys, xs], cv2.CV_64F)
    if mask is not None:
        m = mask[ys, xs].astype(bool)
        lap = lap[m]
    return float(lap.var())


def sharpness_curve(path, roi_frac: float = 0.5, **kw) -> np.ndarray:
    """3.7 A: sharp(t) = variance of the Laplacian over the centre ROI, per frame."""
    return np.array([laplacian_variance(f, roi_frac) for _, f in iter_frames(path, gray=True, **kw)])


@dataclass
class SharpKeypoints:
    onset_frame: int | None       # first frame below 90 % of baseline
    onset_delay_s: float | None   # relative to trigger_frame
    peak_norm: float              # minimum of the normalised curve
    peak_frame: int
    recovery_frame: int | None    # first frame back at >= 90 % after the minimum
    recovery_time_s: float | None  # from the minimum to recovery
    unc_s: float


def sharp_keypoints(sharp: np.ndarray, base_frames: slice | tuple[int, int], fps: float,
                    trigger_frame: int, level: float = 0.9) -> SharpKeypoints:
    """3.7 A step 3/5: normalise by the median baseline, extract delay / peak / recovery."""
    b = base_frames if isinstance(base_frames, slice) else slice(*base_frames)
    norm = np.asarray(sharp, dtype=np.float64) / float(np.median(sharp[b]))
    after = np.where(np.arange(len(norm)) >= trigger_frame)[0]
    below = [i for i in after if norm[i] < level]
    onset = below[0] if below else None
    pk = int(after[np.argmin(norm[after])])
    rec = None
    for i in range(pk, len(norm)):
        if norm[i] >= level:
            rec = i
            break
    return SharpKeypoints(
        onset, None if onset is None else (onset - trigger_frame) / fps,
        float(norm[pk]), pk, rec, None if rec is None else (rec - pk) / fps, 1.0 / fps)


def flow_shift_curve(path, scale: tuple[int, int] = (320, 180), roi_frac: float = 1.0,
                     exclude: Crop | None = None, **kw) -> np.ndarray:
    """3.7 B: per-frame global camera shift (dx, dy) in *scaled* pixels = median of dense Farneback flow.
    `exclude` (W,H,X,Y in scaled pixels) masks moving UI / characters. Returns (frames-1, 2)."""
    prev = None
    out = []
    for _, f in iter_frames(path, gray=True, scale=scale, **kw):
        if prev is not None:
            fl = cv2.calcOpticalFlowFarneback(prev, f, None, 0.5, 3, 15, 3, 5, 1.2, 0)
            ys, xs = _center_roi(*f.shape[:2], roi_frac)
            m = np.ones(f.shape[:2], dtype=bool)
            if exclude:
                w, h, x, y = exclude
                m[y:y + h, x:x + w] = False
            sel = m[ys, xs]
            out.append((float(np.median(fl[ys, xs, 0][sel])), float(np.median(fl[ys, xs, 1][sel]))))
        prev = f
    return np.array(out)


@dataclass
class ShakeMetrics:
    peak_px: float
    p2p_px: float
    rms_px: float
    dominant_hz: float | None
    onset_frame: int | None


def shake_metrics(shift: np.ndarray, fps: float, noise_floor_px: float | None = None) -> ShakeMetrics:
    """Camera position = cumulative sum of the per-frame shifts (high-passed by removing the mean);
    amplitude a(t) = hypot; RMS and peak-to-peak; FFT dominant frequency (3.7 B step 3-4)."""
    pos = np.cumsum(shift, axis=0)
    pos = pos - pos.mean(axis=0)
    a = np.hypot(pos[:, 0], pos[:, 1])
    ax = 0 if np.ptp(pos[:, 0]) >= np.ptp(pos[:, 1]) else 1
    sig = pos[:, ax]
    freqs = np.fft.rfftfreq(len(sig), 1.0 / fps)
    spec = np.abs(np.fft.rfft(sig * np.hanning(len(sig))))
    spec[0] = 0
    dom = float(freqs[int(np.argmax(spec))]) if len(sig) > 8 and spec.max() > 0 else None
    nf = noise_floor_px if noise_floor_px is not None else float(np.median(a)) * 3 + 0.05
    on = np.where(a > nf)[0]
    return ShakeMetrics(float(a.max()), float(np.ptp(sig)), float(np.sqrt(np.mean(sig ** 2))),
                        dom, int(on[0]) if len(on) else None)


def vignette_curve(path, edge_frac: float = 0.15, **kw) -> np.ndarray:
    """3.7 C: mean brightness ratio L_edge / L_center per frame."""
    out = []
    for _, f in iter_frames(path, gray=True, **kw):
        h, w = f.shape
        eh, ew = max(1, int(h * edge_frac)), max(1, int(w * edge_frac))
        edge = np.concatenate([f[:eh].ravel(), f[-eh:].ravel(), f[:, :ew].ravel(), f[:, -ew:].ravel()])
        cy, cx = _center_roi(h, w, 0.3)
        out.append(float(edge.mean()) / max(1e-6, float(f[cy, cx].mean())))
    return np.array(out)


def saturation_curve(path, **kw) -> np.ndarray:
    """3.7 C: mean HSV saturation per frame."""
    out = []
    for _, f in iter_frames(path, gray=False, **kw):
        out.append(float(cv2.cvtColor(f, cv2.COLOR_BGR2HSV)[..., 1].mean()))
    return np.array(out)
