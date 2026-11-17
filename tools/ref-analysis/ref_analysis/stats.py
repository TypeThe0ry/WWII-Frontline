"""Statistics, confidence grade and tolerances (REFERENCE_MEASUREMENTS 2.5, 4.0, 7.3)."""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np


@dataclass(frozen=True)
class Stats:
    n: int
    median: float
    min: float
    max: float
    range: float
    uncertainty: float   # max(range/2, frame uncertainty)
    range_pct: float     # range / |median| * 100 (inf if median == 0 and range > 0)


def summarize(values, frame_unc: float = 0.0, extra_unc: float = 0.0) -> Stats:
    """Median, range and uncertainty = max(range/2, 1 frame) (2.5). `frame_unc` is the +-1 frame
    uncertainty converted to the unit of the values (0 for non-time values); `extra_unc` is a
    propagated uncertainty (e.g. from `speed_unc`)."""
    v = np.asarray([x for x in values if x is not None and not math.isnan(x)], dtype=np.float64)
    if len(v) == 0:
        raise ValueError("no values")
    med = float(np.median(v))
    rng = float(v.max() - v.min())
    unc = max(rng / 2.0, frame_unc, extra_unc)
    pct = (rng / abs(med) * 100.0) if med != 0 else (0.0 if rng == 0 else math.inf)
    return Stats(len(v), med, float(v.min()), float(v.max()), rng, unc, pct)


_LEVELS = "ABCD"


def confidence(
    n: int,
    range_pct: float,
    public_conflict: bool = False,
    cross_validated: bool | None = None,
    infeasible: bool = False,
) -> str:
    """4.0 grades.

    D  infeasible (black-box not measurable; historical / estimate);
    A  n >= 10 and range <= 5 % of the median;
    B  n >= 5 and range <= 20 %;
    C  otherwise (n < 5, range > 20 %).
    Modifiers: a conflict with public sources lowers the grade by one level (A needs "no conflict");
    `cross_validated=False` (only one method, 'C: ... 仅一种方法') caps the grade at C; the default
    None means "not assessed" and changes nothing."""
    if infeasible:
        return "D"
    if n >= 10 and range_pct <= 5.0:
        g = 0
    elif n >= 5 and range_pct <= 20.0:
        g = 1
    else:
        g = 2
    if public_conflict:
        g = min(2, g + 1)
    if cross_validated is False:
        g = 2
    return _LEVELS[g]


def speed_unc(d: float, dd: float, t: float, dt: float) -> float:
    """v = d / t: dv/v = sqrt((dd/d)^2 + (dt/t)^2) (2.5); returns dv in m/s."""
    return (d / t) * math.hypot(dd / d, dt / t)


# ------------------------------------------------------------------------------ 7.3 tolerances

KINDS = ("timing", "speed", "distance", "angle", "stat", "count", "curve", "ratio")
FRAME_60_S = 1.0 / 60.0


def tolerance_abs(kind: str, target: float, ref_unc: float = 0.0) -> float:
    """Absolute tolerance (same unit as `target`) per 7.3, relaxed to the reference's own uncertainty
    when that is larger (last row of the table).

    timing   +-5 % or +-1 frame @60 fps (16.7 ms), the larger        -> `target` in seconds
    speed    +-3 %
    distance +-5 % or +-1 m, the larger                                -> metres
    angle    +-5 % or +-0.1 deg, the larger                            -> degrees
    stat/ratio  median within +-10 %                                   (shape: p95 +-15 % is checked separately)
    count    exact equality (shots to kill)
    curve    key points +-10 %
    """
    a = abs(target)
    if kind == "timing":
        t = max(0.05 * a, FRAME_60_S)
    elif kind == "speed":
        t = 0.03 * a
    elif kind == "distance":
        t = max(0.05 * a, 1.0)
    elif kind == "angle":
        t = max(0.05 * a, 0.1)
    elif kind in ("stat", "ratio", "curve"):
        t = 0.10 * a
    elif kind == "count":
        return 0.0
    else:
        raise ValueError(f"unknown tolerance kind {kind!r}")
    return max(t, abs(ref_unc))


@dataclass(frozen=True)
class Compare:
    error: float
    error_pct: float
    tolerance_abs: float
    tolerance_pct: float
    passed: bool


def compare(kind: str, own: float, target: float, ref_unc: float = 0.0) -> Compare:
    tol = tolerance_abs(kind, target, ref_unc)
    err = own - target
    pct = (err / target * 100.0) if target else (0.0 if err == 0 else math.inf)
    tpct = (tol / abs(target) * 100.0) if target else (0.0 if tol == 0 else math.inf)
    ok = (err == 0) if kind == "count" else (abs(err) <= tol + 1e-12)
    return Compare(err, pct, tol, tpct, ok)
