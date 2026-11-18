"""Off-mesh link format + validation (M1-036). Links are hand/semi-automatic annotations in map data."""
from __future__ import annotations

import math

KINDS = ("window", "ladder", "stairs", "vault")


def validate(links: list, near_fn, max_off_m: float = 0.5) -> list[str]:
    """`near_fn(x, y, z) -> distance to the nearest navmesh point`. Returns violation strings."""
    errs = []
    seen = set()
    for l in links:
        i = l.get("id", "?")
        if i in seen:
            errs.append(f"{i}: duplicate id")
        seen.add(i)
        if l.get("kind") not in KINDS:
            errs.append(f"{i}: bad kind {l.get('kind')!r}")
        for k in ("start", "end"):
            p = l.get(k)
            if not (isinstance(p, list) and len(p) == 3):
                errs.append(f"{i}: {k} must be [x,y,z]")
                continue
            d = near_fn(*p)
            if d > max_off_m:
                errs.append(f"{i}: {k} is {d:.2f} m from the navmesh (> {max_off_m})")
        if not (isinstance(l.get("radius"), (int, float)) and l["radius"] > 0):
            errs.append(f"{i}: radius must be > 0")
        if not isinstance(l.get("bidirectional"), bool):
            errs.append(f"{i}: bidirectional must be bool")
        if not (isinstance(l.get("cost"), (int, float)) and l["cost"] >= 1.0 and math.isfinite(l["cost"])):
            errs.append(f"{i}: cost must be >= 1")
    return errs
