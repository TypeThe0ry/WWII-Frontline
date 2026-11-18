"""Rough modern-feature removal for the OSM draft (M1-032). 1944 fixes are M1-033."""
from __future__ import annotations

from collections import Counter

from . import osm_rules


def split(elements):
    """Return (kept, dropped_counter). `elements` are normalised dicts with `tags`."""
    kept, dropped = [], Counter()
    for e in elements:
        r = osm_rules.modern_reason(e["tags"])
        if r:
            dropped[r] += 1
        else:
            kept.append(e)
    return kept, dict(sorted(dropped.items()))
