"""Tag -> game attribute rules for the OSM draft layers (M1-032, MAPS §6.1)."""
from __future__ import annotations

import re

WIDTH_DEFAULT_M = {"paved": 6.0, "dirt": 4.0, "path": 1.5, "rail": 5.0, "causeway": 6.0}
SERVICE_WIDTH_M = 4.0
STREAM_WIDTH_M = 2.0
WATER_DEFAULTS = {
    "river": {"width_m": 20.0, "depth_m": 2.0},
    "canal": {"width_m": 18.0, "depth_m": 3.0},
    "stream": {"width_m": 2.0, "depth_m": 0.4},
    "pond": {"width_m": None, "depth_m": 1.0},
    "wetland": {"width_m": None, "depth_m": 0.3},
}
WALKABLE_DEPTH_M = 0.5
HEDGE_HEIGHT_M = 2.0
HEDGE_MOUND_M = 1.0
FOREST_DENSITY = 0.6

HIGHWAY_CLASS = {
    "motorway": "paved", "trunk": "paved", "primary": "paved", "secondary": "paved",
    "tertiary": "paved", "unclassified": "paved", "residential": "paved",
    "living_street": "paved", "service": "paved", "primary_link": "paved",
    "secondary_link": "paved", "tertiary_link": "paved", "trunk_link": "paved",
    "motorway_link": "paved",
    "track": "dirt",
    "path": "path", "footway": "path", "cycleway": "path", "pedestrian": "path",
    "steps": "path", "bridleway": "path",
}
CAUSEWAY_CANDIDATES = {"motorway", "trunk", "motorway_link", "trunk_link"}
RAIL_KEEP = {"rail"}
RAIL_DROP = {"abandoned", "razed", "platform", "station", "disused", "construction"}
# modern buildings (M1-032 draft filter; real 1944 pass is M1-033)
MODERN_BUILDING = {"retail", "industrial", "commercial", "supermarket", "warehouse", "roof", "hospital", "school", "train_station", "service"}
MODERN_LANDUSE = {"retail", "industrial", "commercial"}


def _num(s):
    if s is None:
        return None
    m = re.match(r"\s*([0-9]+(?:[.,][0-9]+)?)", str(s))
    return float(m.group(1).replace(",", ".")) if m else None


def is_bridge(tags: dict) -> bool:
    return tags.get("bridge", "no") not in ("no", "", None) and tags.get("man_made") != "bridge"


def road_class(tags: dict):
    """Return (class, derived_list, extras) or None when the way is not a road/rail."""
    hw = tags.get("highway")
    rw = tags.get("railway")
    derived: list[str] = []
    extras: dict = {}
    if hw:
        cls = HIGHWAY_CLASS.get(hw)
        if cls is None:
            return None
        if hw in CAUSEWAY_CANDIDATES:
            extras["causeway_candidate"] = True
        surf = tags.get("surface")
        if surf is None:
            surf = {"paved": "asphalt", "dirt": "ground", "path": "unpaved"}[cls]
            derived.append("surface")
        w = _num(tags.get("width"))
        if w is None:
            w = SERVICE_WIDTH_M if hw == "service" else WIDTH_DEFAULT_M[cls]
            derived.append("width_m")
    elif rw in RAIL_KEEP:
        cls, surf, w = "rail", "ballast", _num(tags.get("width"))
        derived.append("surface")
        if w is None:
            w = WIDTH_DEFAULT_M["rail"]
            derived.append("width_m")
    else:
        return None
    bridge = is_bridge(tags)
    if bridge:
        elev = "bridge"
    elif tags.get("embankment") == "yes" or cls == "rail":
        elev = "embankment" if tags.get("embankment") == "yes" else "terrain"
    else:
        elev = "terrain"
    vehicle_ok = cls in ("paved", "dirt", "causeway") and w >= 3.0
    if tags.get("access") in ("no", "private") and hw != "track":
        vehicle_ok = vehicle_ok and False
    return {"class": cls, "width_m": round(w, 2), "surface": surf, "bridge": bridge,
            "elev_mode": elev, "vehicle_ok": bool(vehicle_ok), "_derived": derived, **extras}


def water_kind(tags: dict):
    """(kind, is_area) or None."""
    ww = tags.get("waterway")
    if ww in ("river", "canal", "stream"):
        return ww, False
    if tags.get("natural") == "water":
        return "pond" if tags.get("water") in (None, "pond", "lake", "basin", "reservoir") else tags["water"], True
    if tags.get("natural") == "wetland":
        return "wetland", True
    return None


def water_props(tags: dict, kind: str) -> dict:
    d = WATER_DEFAULTS.get(kind, WATER_DEFAULTS["pond"])
    derived = ["depth_m"]
    w = _num(tags.get("width"))
    if w is None and d["width_m"] is not None:
        w = d["width_m"]
        derived.append("width_m")
    props = {"kind": kind, "depth_m": d["depth_m"], "walkable_depth_m": WALKABLE_DEPTH_M, "_derived": derived + ["walkable_depth_m"]}
    if w is not None:
        props["width_m"] = w
    return props


def hedge_props(tags: dict) -> dict:
    h = _num(tags.get("height"))
    derived = ["mound_width_m", "gap"]
    if h is None:
        h = HEDGE_HEIGHT_M
        derived.append("height_m")
    return {"height_m": h, "mound_width_m": HEDGE_MOUND_M, "gap": False,
            "vehicle_blocking": h >= 1.8, "source": "osm_draft", "kind": tags.get("barrier") or tags.get("natural"), "_derived": derived + ["vehicle_blocking"]}


def building_props(tags: dict) -> dict:
    derived = ["style", "enterable", "destructible", "asset_ref", "ruin_stage"]
    lv = _num(tags.get("building:levels"))
    if lv is None:
        lv = 1
        derived.append("levels")
    roof = tags.get("roof:shape")
    if roof is None:
        roof = "unknown"
        derived.append("roof")
    return {"style": "unknown", "levels": max(1, int(lv)), "roof": roof, "enterable": False,
            "destructible": True, "asset_ref": None, "ruin_stage": 0,
            "type": tags.get("building", "yes"), "_derived": derived}


def forest_kind(tags: dict):
    if tags.get("landuse") == "forest" or tags.get("natural") == "wood":
        return "forest"
    if tags.get("landuse") == "orchard":
        return "orchard"
    if tags.get("natural") == "scrub":
        return "scrub"
    return None


def forest_props(tags: dict, kind: str) -> dict:
    sp = tags.get("species") or tags.get("leaf_type")
    derived = ["density"] + ([] if sp else ["species"])
    return {"kind": kind, "density": FOREST_DENSITY, "species": sp or "unknown", "_derived": derived}


def modern_reason(tags: dict):
    """Why a feature is dropped as obviously modern/irrelevant, else None."""
    b = tags.get("building")
    if b in MODERN_BUILDING:
        return f"building={b}"
    if "shop" in tags and b:
        return "shop"
    lu = tags.get("landuse")
    if lu in MODERN_LANDUSE:
        return f"landuse={lu}"
    rw = tags.get("railway")
    if rw in RAIL_DROP:
        return f"railway={rw}"
    if tags.get("highway") in ("cycleway",):
        return "highway=cycleway"
    return None
