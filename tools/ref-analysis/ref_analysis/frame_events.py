"""M-FRM: event timing from frame numbers (REFERENCE_MEASUREMENTS 3.2)."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .extract_frames import Crop, iter_frames


def luminance_series(path, roi: Crop | None = None, **kw) -> np.ndarray:
    """Mean brightness per frame over a region (W,H,X,Y) in the source resolution."""
    return np.array([float(f.mean()) for _, f in iter_frames(path, crop=roi, gray=True, **kw)])


def detect_onsets(series: np.ndarray, level: float = 0.5, min_gap_frames: int = 2) -> list[int]:
    """Frame indices where a flash first appears: first frame whose value rises above
    baseline + level*(peak - baseline) (baseline = median, peak = 99.5th percentile).
    A refractory period of `min_gap_frames` frames skips the tail of the same flash."""
    s = np.asarray(series, dtype=np.float64)
    base = float(np.median(s))
    peak = float(np.percentile(s, 99.5))
    if peak - base < 1e-9:
        return []
    thr = base + level * (peak - base)
    onsets: list[int] = []
    above_prev = False
    last = -10**9
    for i, v in enumerate(s):
        above = v > thr
        if above and not above_prev and i - last >= min_gap_frames:
            onsets.append(i)
            last = i
        above_prev = above
    return onsets


@dataclass
class FrameRate:
    n: int
    first: int
    last: int
    rpm: float
    mean_interval_s: float
    uncertainty_rpm: float


def rpm_from_frames(onsets: list[int], fps: float) -> FrameRate:
    """RPM = 60 (N-1) / ((n_last - n_first)/fps); uncertainty from +-1 frame on the span (3.2 step 4)."""
    if len(onsets) < 2:
        return FrameRate(len(onsets), -1, -1, float("nan"), float("nan"), float("nan"))
    span = onsets[-1] - onsets[0]
    n = len(onsets)
    rpm = 60.0 * (n - 1) * fps / span
    return FrameRate(n, onsets[0], onsets[-1], rpm, span / fps / (n - 1), rpm / span)


def event_delta(frame_a: int, frame_b: int, fps: float) -> tuple[float, float]:
    """Delta t between two event frames and its +-1 frame uncertainty."""
    return (frame_b - frame_a) / fps, 1.0 / fps
