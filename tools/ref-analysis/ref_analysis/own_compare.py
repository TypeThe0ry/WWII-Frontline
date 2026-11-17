"""Own-game re-measurement side (REFERENCE_MEASUREMENTS 7.4): load the JSON exported by
`cargo test -p frontline-sim --test own_remeasure`, fill summary.csv columns own_median / own_error_pct /
tolerance_pct / pass, and render the table of our own numbers."""

from __future__ import annotations

import json
from pathlib import Path

from .items import by_id
from .record import RecordError, read_summary, validate_row, write_summary
from .stats import compare


def load_own(path: str | Path) -> list[dict]:
    j = json.loads(Path(path).read_text(encoding="utf-8"))
    return j["entries"] if isinstance(j, dict) else j


def fill_own(summary_path: str | Path, own_path: str | Path) -> int:
    """Compare every measured row with our value of the same id. Rows without an own value, or whose
    own value is 'unavailable', get pass='na'. Returns the number of rows updated."""
    own = {e["id"]: e for e in load_own(own_path)}
    cat = by_id()
    rows = read_summary(summary_path)
    changed = 0
    for r in rows:
        if r["status"] != "measured":
            continue
        e = own.get(r["id"])
        if e is None or e.get("median") is None:
            new = {"own_median": "", "own_error_pct": "", "tolerance_pct": "", "pass": "na"}
        else:
            if e.get("unit") and e["unit"] != r["unit"]:
                raise RecordError(f"{r['id']}: unit mismatch own {e['unit']!r} vs ref {r['unit']!r}")
            kind = cat[r["id"]][0].kind if r["id"] in cat else "stat"
            c = compare(kind, float(e["median"]), float(r["median"]), float(r["uncertainty"] or 0))
            new = {
                "own_median": e["median"], "own_error_pct": c.error_pct, "tolerance_pct": c.tolerance_pct,
                "pass": "yes" if c.passed else "no",
            }
            if e.get("data_value") is not None:
                new["data_value"] = e["data_value"]
        merged = validate_row({**r, **new})
        if merged != r:
            r.update(merged)
            changed += 1
    write_summary(summary_path, rows)
    return changed


def own_table_md(entries: list[dict]) -> str:
    L = ["| ID | 项目 | 我方实测中位数 | 单位 | 范围 [min, max] | n | /data 当前值 | 方法 | 备注 |",
         "|---|---|---|---|---|---|---|---|---|"]
    for e in entries:
        med = "不可测" if e.get("median") is None else f"{e['median']:.6g}"
        rng = "" if e.get("median") is None else f"[{e['min']:.6g}, {e['max']:.6g}]"
        dv = "" if e.get("data_value") is None else f"{e['data_value']:.6g}"
        L.append(f"| {e['id']} | {e.get('item', '')} | {med} | {e.get('unit', '')} | {rng} | {e.get('n', '')} | "
                 f"{dv} | {e.get('method', '')} | {str(e.get('notes', '')).replace('|', '/')} |")
    return "\n".join(L) + "\n"
