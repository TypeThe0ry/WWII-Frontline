"""Carentan ``meta.json`` (MAPS §6.2): anchors, similarity, sources, credits (M1-030).

Everything here is deterministic: the same repo state gives byte-identical output. Real positions
for anchors other than the church are the *provisional* design-frame values (MAPS §3.4 gives no
coordinate source for them); each is flagged ``provisional`` until a human confirms it (H-051).
"""
from __future__ import annotations

import json
from pathlib import Path

from . import TOOL_VERSION, anchors as an, geo, licenses as lic

ORIGIN_E, ORIGIN_N, SIZE_M, EPSG, ZONE = 626432, 5462016, 2048, 32630, 30

# id, design (x, y), confidence_m, source, real (E, N) or None (= provisional frame value)
ANCHORS = [
    ("church_notre_dame", (1277, 897), 40, "cirkwi+merimee:PA00110352", (627709, 5462913)),
    ("place_republique", (1380, 990), 120, "maps_s3.4_provisional", None),
    ("canal_lock", (1200, 1350), 120, "maps_s3.4_provisional", None),
    ("n13_town_entry", (900, 1150), 150, "maps_s3.4_provisional", None),
    ("n13_nw_corner", (0, 1980), 300, "maps_s3.4_provisional", None),
    ("railway_station", (1500, 520), 250, "maps_s3.4_provisional", None),
    ("ingouf_farm", (900, 1450), 250, "maps_s3.4_provisional", None),
    ("flood_east_edge", (1650, 1500), 300, "maps_s3.4_provisional", None),
]

SOURCES = [
    dict(
        id="ign_lidar_hd", asset_id="AST-M-LIDAR-01", req_id="M-CAR-001", license="Etalab-2.0",
        name="IGN LiDAR HD (MNT/MNS 0.5 m)", channel="IGN / cartes.gouv.fr", vendor="IGN",
        url="https://cartes.gouv.fr/rechercher-une-donnee/dataset/IGNF_NUAGES-DE-POINTS-LIDAR-HD",
        license_name="Licence Ouverte / Etalab 2.0",
        notes="待 H-052 核实：门户页抓取为空壳，许可证来自搜索摘要；须确认 Calvados/Manche 覆盖年份",
        credit_text="Source : IGN, LiDAR HD (Licence Ouverte Etalab 2.0)",
    ),
    dict(
        id="ign_rge_alti", asset_id="AST-M-ALTI-01", req_id="M-CAR-002", license="Etalab-2.0",
        name="IGN RGE ALTI 1 m", channel="IGN / cartes.gouv.fr", vendor="IGN",
        url="https://cartes.gouv.fr/rechercher-une-donnee/dataset/IGNF_RGE-ALTI",
        license_name="Licence Ouverte / Etalab 2.0",
        notes="待 H-052 核实：LiDAR 覆盖缺口备选；许可证来自第三方目录与搜索摘要",
        credit_text="Source : IGN, RGE ALTI (Licence Ouverte Etalab 2.0)",
    ),
    dict(
        id="copernicus_glo30", asset_id="AST-M-GLO30-01", req_id="M-CAR-003", license="Copernicus-GLO-30",
        name="Copernicus DEM GLO-30", channel="Copernicus Data Space", vendor="DLR e.V. / Airbus",
        url="https://documentation.dataspace.copernicus.eu/APIs/SentinelHub/Data/DEM/resources/license/License-COPDEM-30.pdf",
        license_name="Copernicus DEM licence (Full, Free & Open)",
        notes="待 H-052 核实：远景环外圈兜底；须读许可证原文与国家例外清单",
        credit_text="Copernicus DEM GLO-30 (Copernicus WorldDEM-30) © DLR e.V.",
    ),
    dict(
        id="osm", asset_id="AST-M-OSM-01", req_id="M-CAR-004", license="ODbL-1.0",
        name="OpenStreetMap extract (roads/buildings/water)", channel="OpenStreetMap", vendor="OpenStreetMap contributors",
        url="https://www.openstreetmap.org/copyright", license_name="ODbL 1.0",
        notes="待 H-052/H-004：衍生矢量层按 ODbL 公开（BRIEF R9 默认），入库前需法律确认",
        credit_text="© OpenStreetMap contributors",
    ),
]


def anchor_list() -> list[an.Anchor]:
    out = []
    for aid, (dx, dy), conf, _src, real in ANCHORS:
        r = geo.to_local(*real, ORIGIN_E, ORIGIN_N) if real else (float(dx), float(dy))
        out.append(an.Anchor(aid, (float(dx), float(dy)), r, float(conf)))
    return out


def build_meta(root: Path, ledger_text: str | None = None) -> dict:
    ledger = lic.parse_ledger(ledger_text if ledger_text is not None else _ledger(root))
    anchors = []
    for aid, design, conf, src, real in ANCHORS:
        e, n = real if real else (ORIGIN_E + design[0], ORIGIN_N + design[1])
        anchors.append(
            {"id": aid, "e": e, "n": n, "design": list(design), "confidence_m": conf, "source": src,
             "provisional": real is None}
        )
    sim = an.fit_similarity(anchor_list())
    sources = []
    for s in SOURCES:
        archive = f"licenses/texts/{s['asset_id']}.txt"
        row = ledger.get(s["asset_id"])
        verified = None
        if row and row.get("status") == "OK" and lic.archive_exists(root, archive):
            verified = row.get("purchase_date") or None
        sources.append(
            {"id": s["id"], "asset_id": s["asset_id"], "license": s["license"], "verified": verified,
             "archive": archive}
        )
    return {
        "schema": "frontline.map.meta/1",
        "map_id": "carentan",
        "display_name": "Carentan",
        "date_setting": "1944-06-11",
        "crs": {"epsg": EPSG, "origin_e": ORIGIN_E, "origin_n": ORIGIN_N, "size_m": SIZE_M},
        "height": {"res_m": 1, "samples": SIZE_M + 1, "hmin_m": None, "hmax_m": None, "datum": "IGN69",
                   "status": "placeholder (M1-031)"},
        "far_ring": {"res_m": 8, "samples": 1025, "extent_m": 8192},
        "anchors": anchors,
        "similarity": {"dx": round(sim["dx"], 3), "dy": round(sim["dy"], 3),
                       "rot_deg": round(sim["rot_deg"], 4), "applied": False,
                       "mode": sim["mode"], "rms_m": round(sim["rms_m"], 3)},
        "sources": sources,
        "credits": [s["credit_text"] for s in SOURCES],
        "m1_sources_checked": False,
        "tool_version": TOOL_VERSION,
    }


def _ledger(root: Path) -> str:
    p = root / "LICENSES.md"
    return p.read_text(encoding="utf-8") if p.is_file() else ""


def validate_meta(meta: dict) -> list[str]:
    """Structural checks (MAPS §6.2); returns error strings."""
    errs = []
    for k in ("schema", "map_id", "crs", "height", "far_ring", "anchors", "similarity", "sources"):
        if k not in meta:
            errs.append(f"meta: missing '{k}'")
    if errs:
        return errs
    if meta["schema"] != "frontline.map.meta/1":
        errs.append("meta.schema must be frontline.map.meta/1")
    anchors = meta["anchors"]
    if len(anchors) < 8:
        errs.append(f"meta.anchors: need >= 8, have {len(anchors)}")
    if sum(1 for a in anchors if a.get("confidence_m", 1e9) <= 150) < 3:
        errs.append("meta.anchors: need >= 3 anchors with confidence_m <= 150")
    for a in anchors:
        for k in ("id", "e", "n", "design", "confidence_m", "source"):
            if k not in a:
                errs.append(f"anchor {a.get('id', '?')}: missing '{k}'")
        if not a.get("confidence_m", 0) > 0:
            errs.append(f"anchor {a.get('id', '?')}: confidence_m must be > 0")
    sim = meta["similarity"]
    if abs(sim.get("rot_deg", 0)) > an.MAX_ROT_DEG:
        errs.append("meta.similarity.rot_deg exceeds 10 deg")
    for s in meta["sources"]:
        if s.get("verified") and not s.get("archive"):
            errs.append(f"source {s.get('id')}: verified without archive")
    return errs


def dump(path: Path, obj: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
