"""Map data-source licence check against the LICENSES.md §3 ledger (M1-030, MAPS §2).

The ledger is the only authority: a source is ``OK`` only when its row says ``OK`` *and* the
licence text is archived under ``licenses/texts/``. This module never writes ``OK`` on its own and
never edits LICENSES.md; it reports gaps and drafts rows for the human (H-052).
"""
from __future__ import annotations

import json
from pathlib import Path

OK, PENDING, MISSING = "OK", "PENDING", "MISSING"


def parse_ledger(text: str) -> dict[str, dict[str, str]]:
    """Rows of every markdown table whose header starts with ``asset_id``, keyed by asset_id.
    ``AST-EXAMPLE-`` rows are ignored (LICENSES.md §2)."""
    rows: dict[str, dict[str, str]] = {}
    header: list[str] | None = None
    for line in text.splitlines():
        s = line.strip()
        if not s.startswith("|"):
            header = None if not s else header
            continue
        cells = [c.strip() for c in s.strip("|").split("|")]
        if cells and cells[0] == "asset_id":
            header = cells
            continue
        if header is None or set("".join(cells)) <= set("-: "):
            continue
        row = dict(zip(header, cells))
        aid = row.get("asset_id", "")
        if aid and not aid.startswith("AST-EXAMPLE-"):
            rows[aid] = row
    return rows


def archive_exists(root: Path, rel: str | None) -> bool:
    if not rel:
        return False
    p = root / rel
    if p.is_file():
        return True
    return any(p.parent.glob(p.stem + ".*")) if p.parent.is_dir() else False


def check_sources(meta: dict, ledger: dict[str, dict[str, str]], root: Path) -> list[dict]:
    out = []
    for src in meta.get("sources", []):
        aid, why = src.get("asset_id", ""), []
        row = ledger.get(aid)
        if row is None:
            status = MISSING
            why.append("no ledger row in LICENSES.md §3")
        else:
            archived = archive_exists(root, src.get("archive"))
            if row.get("status") == "OK" and archived:
                status = OK
            else:
                status = PENDING
                if row.get("status") != "OK":
                    why.append(f"ledger status is {row.get('status') or '?'}")
                if not archived:
                    why.append(f"licence text not archived at {src.get('archive')}")
        verified = src.get("verified")
        if status == OK and not verified:
            why.append("OK in ledger but meta.verified is null (set from purchase_date)")
        if status != OK and verified:
            status = PENDING
            why.append("meta.verified is set but the ledger does not back it")
        out.append({"id": src["id"], "asset_id": aid, "status": status, "reasons": why})
    return out


def false_ok(report: list[dict]) -> list[str]:
    """Ids reported OK without a ``why``-free backing: used by the test as a guard."""
    return [r["id"] for r in report if r["status"] == OK and r["reasons"]]


def write_report(path: Path, report: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    body = {"schema": "frontline.map.license_report/1", "sources": report}
    path.write_text(json.dumps(body, indent=2, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")


def draft_rows(sources: list[dict]) -> str:
    """Markdown with ledger rows for the human to paste into LICENSES.md §3 (status PENDING)."""
    cols = [
        "asset_id", "req_id", "name", "type", "source_channel", "author_vendor", "url",
        "license_name_version", "license_text_path", "chk_engine", "chk_web", "chk_commercial",
        "chk_attribution", "chk_seats", "order_id", "purchase_date", "purchaser", "confirmation_path",
        "raw_path", "processed_package", "allowed_processing", "status", "notes", "credit_text",
    ]
    lines = [
        "# 地图数据源台账草稿行（M1-030，给人类粘贴到 LICENSES.md §3）",
        "",
        "全部为 `PENDING`：许可证来自搜索摘要/第三方目录，**未经官方页面核实**（MAPS §2.1/§2.5）。",
        "H-052 核实后：存档许可证全文到 `license_text_path`，把 `chk_*` 与 `status` 改为实际结论、",
        "补 `purchase_date`（= 核实日期）。执行者不得自行写 OK。",
        "",
        "| " + " | ".join(cols) + " |",
        "|" + "---|" * len(cols),
    ]
    for s in sources:
        r = {c: "—" for c in cols}
        r.update(
            asset_id=s["asset_id"], req_id=s["req_id"], name=s["name"], type="other",
            source_channel=s["channel"], author_vendor=s["vendor"], url=s["url"],
            license_name_version=s["license_name"], license_text_path=f"licenses/texts/{s['asset_id']}.txt",
            chk_engine="PENDING", chk_web="PENDING", chk_commercial="PENDING",
            chk_attribution="REQUIRED", chk_seats="PENDING", order_id="PENDING-H-052",
            status="PENDING", notes=s["notes"], credit_text=s["credit_text"],
        )
        lines.append("| " + " | ".join(r[c].replace("|", "/") for c in cols) + " |")
    return "\n".join(lines) + "\n"
