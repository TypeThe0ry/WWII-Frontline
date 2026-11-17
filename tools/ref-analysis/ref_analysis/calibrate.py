"""M-CAL / M-TPL / M-ANG: pixel and angle calibration, tracking, velocity profiles
(REFERENCE_MEASUREMENTS 3.4, 3.5, 3.6)."""

from __future__ import annotations

import math
from dataclasses import dataclass

import cv2
import numpy as np
from scipy.optimize import least_squares

from .extract_frames import Crop, iter_frames


# ---------------------------------------------------------------- M-CAL-D: marks (grid / landmarks)

@dataclass
class SpeedResult:
    speed_mps: float
    uncertainty_mps: float
    dt_s: float


def speed_from_marks(
    frame_a: int, frame_b: int, distance_m: float, fps: float, distance_unc_m: float = 0.0,
) -> SpeedResult:
    """v = d_grid / dt between two event frames (3.4 M-CAL-D step 2). Error propagation:
    dv/v = sqrt((dd/d)^2 + (dt/t)^2) with dt = 1 frame."""
    dt = (frame_b - frame_a) / fps
    if dt <= 0:
        raise ValueError("frame_b must be after frame_a")
    v = distance_m / dt
    rel = math.hypot(distance_unc_m / distance_m, (1.0 / fps) / dt)
    return SpeedResult(v, v * rel, dt)


# ---------------------------------------------------------------- M-TPL: tracking

def track_template(
    path, template: np.ndarray, roi: Crop | None = None, refine: bool = True, **kw,
) -> np.ndarray:
    """Normalised cross-correlation template track. Returns (frames, 3): index, x, y of the template
    centre in source pixels (sub-pixel via parabola fit on the correlation peak)."""
    th, tw = template.shape[:2]
    ox, oy = (roi[2], roi[3]) if roi else (0, 0)
    out = []
    for i, f in iter_frames(path, crop=roi, gray=True, **kw):
        r = cv2.matchTemplate(f, template, cv2.TM_CCOEFF_NORMED)
        _, _, _, (mx, my) = cv2.minMaxLoc(r)
        sx = sy = 0.0
        if refine and 0 < mx < r.shape[1] - 1 and 0 < my < r.shape[0] - 1:
            def par(a, b, c):
                d = a - 2 * b + c
                return 0.0 if abs(d) < 1e-12 else 0.5 * (a - c) / d
            sx = par(r[my, mx - 1], r[my, mx], r[my, mx + 1])
            sy = par(r[my - 1, mx], r[my, mx], r[my + 1, mx])
        out.append((i, ox + mx + sx + tw / 2.0, oy + my + sy + th / 2.0))
    return np.array(out)


def track_bright_blob(path, roi: Crop | None = None, thresh: int = 128, **kw) -> np.ndarray:
    """Intensity-weighted centroid of pixels above `thresh` (for high-contrast markers). (frames,3)."""
    ox, oy = (roi[2], roi[3]) if roi else (0, 0)
    out = []
    for i, f in iter_frames(path, crop=roi, gray=True, **kw):
        m = f.astype(np.float64) - thresh
        m[m < 0] = 0
        s = m.sum()
        if s <= 0:
            out.append((i, float("nan"), float("nan")))
            continue
        ys, xs = np.mgrid[0:f.shape[0], 0:f.shape[1]]
        out.append((i, ox + float((m * xs).sum() / s) + 0.5, oy + float((m * ys).sum() / s) + 0.5))
    return np.array(out)


def fit_speed(frames: np.ndarray, pos_px: np.ndarray, fps: float, px_per_m: float) -> float:
    """Robust (Theil-Sen) slope of position vs time in m/s; ignores NaNs."""
    ok = ~np.isnan(pos_px)
    t = frames[ok] / fps
    x = pos_px[ok] / px_per_m
    if len(t) < 3:
        raise ValueError("need >= 3 samples")
    # subsample pairs to bound the cost on long tracks
    idx = np.arange(len(t))
    k = max(1, len(t) // 400)
    i, j = np.meshgrid(idx[::k], idx[::k])
    m = j > i
    slopes = (x[j[m]] - x[i[m]]) / (t[j[m]] - t[i[m]])
    return float(np.median(slopes))


def speed_series(frames, pos_m, fps: float, win: int = 5) -> np.ndarray:
    """Local speed (m/s) by a +-win/2 frame linear fit; same length as input."""
    t = np.asarray(frames, dtype=np.float64) / fps
    x = np.asarray(pos_m, dtype=np.float64)
    h = win // 2
    v = np.full(len(x), np.nan)
    for i in range(len(x)):
        a, b = max(0, i - h), min(len(x), i + h + 1)
        if b - a >= 3:
            v[i] = np.polyfit(t[a:b], x[a:b], 1)[0]
    return v


@dataclass
class ProfileResult:
    steady_mps: float
    accel_time_s: float | None   # rest -> 95 % of steady speed (M-24)
    stop_time_s: float | None    # release -> speed <= 5 % of steady (M-25)
    stop_distance_m: float | None


def accel_time_95(speed: np.ndarray, fps: float, steady: float | None = None) -> tuple[float, float]:
    """First time the speed reaches 95 % of the steady value. Returns (t_start, t_95) in s, where
    t_start is the last frame with speed <= 5 % of steady before the rise (movement onset)."""
    s = np.nan_to_num(speed)
    steady = steady if steady is not None else float(np.percentile(s, 90))
    hit = int(np.argmax(s >= 0.95 * steady))
    start = hit
    while start > 0 and s[start] > 0.05 * steady:
        start -= 1
    return start / fps, hit / fps


def stop_time(speed: np.ndarray, release_frame: int, fps: float, steady: float) -> float:
    """Time from key release to speed <= 5 % of the steady speed (M-25)."""
    s = np.nan_to_num(speed)
    for i in range(release_frame, len(s)):
        if s[i] <= 0.05 * steady:
            return (i - release_frame) / fps
    return float("nan")


# ---------------------------------------------------------------- M-ANG: FOV and angle calibration

@dataclass
class FovFit:
    fov_deg: float
    focal_px: float
    theta0_deg: float
    rms_px: float
    px_per_deg_center: float


def fov_from_pairs(x_px, theta_deg, width_px: int) -> FovFit:
    """Fit x = f * tan(theta - theta0) + W/2 (3.5 step 4) -> horizontal FOV = 2 atan(W / 2f)."""
    x = np.asarray(x_px, dtype=np.float64)
    th = np.radians(np.asarray(theta_deg, dtype=np.float64))
    c = width_px / 2.0

    def res(p):
        f, t0 = p
        return f * np.tan(th - t0) + c - x

    f0 = (x.max() - x.min()) / max(1e-9, np.tan(th.max()) - np.tan(th.min()))
    s = least_squares(res, [abs(f0) if np.isfinite(f0) and f0 else width_px / 2.0, float(th.mean())])
    f, t0 = float(s.x[0]), float(s.x[1])
    return FovFit(
        fov_deg=math.degrees(2 * math.atan((width_px / 2.0) / abs(f))),
        focal_px=abs(f), theta0_deg=math.degrees(t0),
        rms_px=float(np.sqrt(np.mean(s.fun ** 2))),
        px_per_deg_center=abs(f) * math.pi / 180.0,
    )


def px_to_deg(dpx: float, px_per_deg: float) -> float:
    """3.5: recoil_deg = dy / px_per_deg."""
    return dpx / px_per_deg


def drop_m(angle_deg: float, distance_m: float) -> float:
    """3.4 M-CAL-F: drop = tan(theta) * d."""
    return math.tan(math.radians(angle_deg)) * distance_m


def zoom_ratio(fov_hip_deg: float, fov_ads_deg: float) -> float:
    """ADS magnification as the ratio of tangents of half-FOVs (3.5 step 5, geometric approximation)."""
    return math.tan(math.radians(fov_hip_deg) / 2) / math.tan(math.radians(fov_ads_deg) / 2)


# ---------------------------------------------------------------- transitions (M-07/08, M-12..14)

@dataclass
class Transition:
    start_frame: int
    end_frame: int
    duration_s: float
    uncertainty_s: float


def transition_time(series, fps: float, tol_frac: float = 0.03, plateau: int = 5) -> Transition:
    """Duration of a monotone-ish transition of a scalar (horizon y, bar scale, eye-height proxy ...)
    from its initial plateau to its final plateau: from the first frame deviating more than
    `tol_frac` of the total change from the initial value to the first frame within `tol_frac` of the
    final value. Plateaus = median of the first / last `plateau` frames. +-1 frame uncertainty on
    each end, combined conservatively as +-1 frame (2.2)."""
    s = np.asarray(series, dtype=np.float64)
    a = float(np.median(s[:plateau]))
    b = float(np.median(s[-plateau:]))
    tot = b - a
    if abs(tot) < 1e-9:
        raise ValueError("no change in series")
    u = (s - a) / tot  # 0 -> 1
    start = int(np.argmax(np.abs(u) > tol_frac))
    # end: first frame after start from which the series stays within tol of the final value
    end = len(s) - 1
    for i in range(len(s) - 1, start - 1, -1):
        if abs(u[i] - 1.0) > tol_frac:
            end = min(i + 1, len(s) - 1)
            break
    return Transition(start, end, (end - start) / fps, 1.0 / fps)
