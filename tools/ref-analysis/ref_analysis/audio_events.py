"""M-AUD: audio waveform peak method for rate of fire (REFERENCE_MEASUREMENTS 3.3)."""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
from scipy.io import wavfile
from scipy.ndimage import uniform_filter1d


def load_wav(path) -> tuple[np.ndarray, int]:
    rate, x = wavfile.read(str(path))
    if x.ndim > 1:
        x = x.mean(axis=1)
    if x.dtype.kind == "i":
        x = x.astype(np.float64) / np.iinfo(x.dtype).max
    elif x.dtype.kind == "u":
        x = (x.astype(np.float64) - 128.0) / 128.0
    else:
        x = x.astype(np.float64)
    return x, int(rate)


def envelope(x: np.ndarray, rate: int, win_ms: float = 1.0) -> np.ndarray:
    """Moving RMS of the waveform (3.3 step 3: 1 ms window)."""
    n = max(1, int(round(rate * win_ms / 1000.0)))
    return np.sqrt(uniform_filter1d(np.asarray(x, dtype=np.float64) ** 2, size=n, mode="nearest"))


@dataclass
class AudioResult:
    times_s: np.ndarray
    n: int
    rpm: float
    first_s: float
    last_s: float
    intervals_s: np.ndarray
    params: dict = field(default_factory=dict)

    @property
    def rpm_uncertainty(self) -> float:
        """+-(2 x timing error) over the span; timing error ~ envelope window (conservative 1.5 ms)."""
        span = self.last_s - self.first_s
        if span <= 0:
            return float("nan")
        return self.rpm * (2 * 1.5e-3) / span


def _noise_floor(env: np.ndarray) -> tuple[float, float]:
    med = float(np.median(env))
    mad = float(np.median(np.abs(env - med))) * 1.4826
    return med, mad


def _segments(above: np.ndarray) -> list[tuple[int, int]]:
    """(start, end) sample ranges of consecutive True runs."""
    d = np.diff(np.concatenate([[0], above.astype(np.int8), [0]]))
    return list(zip(np.where(d == 1)[0], np.where(d == -1)[0]))


def _merge(segs: list[tuple[int, int]], gap: int) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    for a, b in segs:
        if out and a - out[-1][1] < gap:
            out[-1] = (out[-1][0], b)
        else:
            out.append((a, b))
    return out


def detect_events(
    x: np.ndarray,
    rate: int,
    expected_interval_s: float | None = None,
    win_ms: float = 1.0,
    thresh_frac: float = 0.25,
    noise_k: float = 8.0,
    min_gap_frac: float = 0.5,
    onset_frac: float = 0.3,
    mode: str = "onset",
) -> AudioResult:
    """Detect shot events and return their times (3.3 step 3).

    The 1 ms RMS envelope is thresholded at max(noise + noise_k*sigma, thresh_frac * 99.9th percentile).
    Above-threshold runs closer than `min_gap_frac` x the shot interval are merged into one event (echo /
    reverb secondary peaks are removed this way). Without `expected_interval_s` it is estimated from the
    data in two refinement passes. `mode='onset'` returns the time the envelope rises above `onset_frac` of
    the event's peak (insensitive to the decay shape); `mode='peak'` returns the envelope maximum."""
    env = envelope(x, rate, win_ms)
    floor, sigma = _noise_floor(env)
    gmax = float(np.percentile(env, 99.9))
    height = max(floor + noise_k * sigma, thresh_frac * gmax)
    raw = _segments(env > height)

    def events(interval_s: float) -> list[tuple[int, int]]:
        return _merge(raw, max(1, int(min_gap_frac * interval_s * rate)))

    guess = expected_interval_s
    if guess is None:
        segs = _merge(raw, int(0.015 * rate))
        guess = 0.05
        for _ in range(3):
            if len(segs) < 3:
                break
            st = np.array([a for a, _ in segs], dtype=np.float64) / rate
            gaps = np.diff(st)
            big = gaps[gaps > gaps.max() * 0.5]
            guess = float(np.median(big))
            segs = events(guess)
    segs = events(guess)

    times = []
    for a, b in segs:
        p = a + int(np.argmax(env[a:b]))
        if mode == "peak":
            times.append(p / rate)
            continue
        thr = onset_frac * env[p]
        i = p
        while i > 0 and env[i] > thr:
            i -= 1
        times.append((i + 1) / rate)
    t = np.asarray(times, dtype=np.float64)
    return summarize_times(t, dict(
        win_ms=win_ms, thresh_frac=thresh_frac, noise_k=noise_k, min_gap_frac=min_gap_frac,
        onset_frac=onset_frac, mode=mode, height=height, expected_interval_s=guess))


def summarize_times(t: np.ndarray, params: dict | None = None, drop_first: int = 0) -> AudioResult:
    t = np.asarray(t, dtype=np.float64)[drop_first:]
    if len(t) < 2:
        return AudioResult(t, len(t), float("nan"), float("nan"), float("nan"), np.array([]), params or {})
    rpm = 60.0 * (len(t) - 1) / (t[-1] - t[0])  # 3.3 step 4
    return AudioResult(t, len(t), rpm, float(t[0]), float(t[-1]), np.diff(t), params or {})


def rpm_from_times(t) -> float:
    return summarize_times(np.asarray(t)).rpm


def audio_rpm(path, **kw) -> AudioResult:
    x, rate = load_wav(path)
    drop = kw.pop("drop_first", 0)
    r = detect_events(x, rate, **kw)
    if drop:
        r = summarize_times(r.times_s, r.params, drop_first=drop)
    return r
