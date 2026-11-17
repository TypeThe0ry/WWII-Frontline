import csv
import re
from pathlib import Path

import pytest

from ref_analysis import record, stats
from ref_analysis.items import M0_ITEMS, all_rows, by_id
from ref_analysis.own_compare import fill_own, own_table_md

REPO = Path(__file__).resolve().parents[3]
DOC = REPO / "docs" / "REFERENCE_MEASUREMENTS.md"
TEMPLATE = REPO / "docs" / "measurements" / "_TEMPLATE.md"
SUMMARY = REPO / "docs" / "measurements" / "summary.csv"


def doc_columns() -> list[str]:
    """Column names parsed from the 8.2 table of the document itself."""
    txt = DOC.read_text(encoding="utf-8")
    sec = txt[txt.index("### 8.2"):txt.index("### 8.3")]
    return re.findall(r"^\| `([a-z_]+)` \|", sec, flags=re.M)


def test_summary_columns_match_doc_8_2_exactly():
    cols = doc_columns()
    assert len(cols) == 25
    assert record.SUMMARY_COLUMNS == cols


def test_committed_summary_csv_has_exact_header_and_validates():
    with SUMMARY.open(newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
    assert header == doc_columns() and len(header) == 25
    record.validate_summary(SUMMARY)


def test_template_is_the_8_1_block_verbatim():
    txt = DOC.read_text(encoding="utf-8")
    sec = txt[txt.index("### 8.1"):txt.index("### 8.2")]
    block = re.search(r"````markdown\n(.*?)\n````", sec, flags=re.S).group(1)
    assert TEMPLATE.read_text(encoding="utf-8").rstrip("\n") == block.rstrip("\n")


def _stats():
    return stats.summarize([600.0, 604.0, 598.0, 602.0, 601.0], frame_unc=0.0)


def _row(item_id="W-01.us_m1_garand", version="synthetic-0", **kw):
    it, sid = by_id()[item_id]
    base = record.row_from_stats(it, item_id, sid, _stats(), fps=120, method="M-AUD", scene="S-RANGE",
                                 ref_version=version, date="2026-11-17", confidence="B")
    base.update(kw)
    return base


def test_row_validation(tmp_path):
    r = record.validate_row(_row())
    assert list(r) == record.SUMMARY_COLUMNS
    with pytest.raises(record.RecordError):
        record.validate_row(_row(status="done"))
    with pytest.raises(record.RecordError):
        record.validate_row(_row(notes="a\nb"))
    with pytest.raises(record.RecordError):
        record.validate_row({**_row(), "bogus": 1})
    with pytest.raises(record.RecordError):
        record.validate_row(_row(median=""))       # measured row needs numbers
    with pytest.raises(record.RecordError):
        record.validate_row(_row(median=float("nan")))


def test_upsert_is_idempotent_and_keyed_by_id_and_version(tmp_path):
    p = tmp_path / "summary.csv"
    record.write_summary(p, [])
    assert record.upsert_summary_row(p, _row()) is True
    first = p.read_text(encoding="utf-8")
    assert record.upsert_summary_row(p, _row()) is False   # no change
    assert p.read_text(encoding="utf-8") == first
    assert record.upsert_summary_row(p, _row(notes="updated")) is True
    assert len(record.read_summary(p)) == 1
    record.upsert_summary_row(p, _row(version="synthetic-1"))   # new version -> new row (stale rows kept)
    assert len(record.read_summary(p)) == 2
    rows = record.validate_summary(p)
    assert all(len(r) == 25 for r in rows)
    # commas / quotes in notes survive a round trip
    record.upsert_summary_row(p, _row(notes='a, "b"'))
    assert record.read_summary(p)[0]["notes"] == 'a, "b"'


def test_header_mismatch_is_rejected(tmp_path):
    p = tmp_path / "bad.csv"
    p.write_text("id,category\nW-01,x\n", encoding="utf-8")
    with pytest.raises(record.RecordError):
        record.read_summary(p)


def test_record_file_rendering_from_synthetic_data(tmp_path):
    st = _stats()
    spec = record.RecordSpec(
        id="W-01.us_m1_garand", name="实际射速",
        meta={"version": "synthetic-0", "date": "2026-11-17", "capture": "120 fps / 1920x1080 / 80 Mbps",
              "frame_unc": "8.33 ms", "scene": "S-RANGE", "subject": "us_m1_garand", "method": "M-AUD"},
        reps=[record.Rep(f"r{i + 1:02d}", f"synthetic_r{i + 1:02d}.mkv", 10, 400, 31, v, "发/分")
              for i, v in enumerate([600.0, 604.0, 598.0, 602.0, 601.0])],
        stats={"n": st.n, "median": st.median, "min": st.min, "max": st.max, "uncertainty": st.uncertainty,
               "unit": "发/分", "confidence": "B", "cross_validation": "一致"},
        mapped=["`weapons/{w}.json:fire.rpm` = 600 ; source=ref_measurement"])
    p = record.write_record(tmp_path, spec)
    txt = p.read_text(encoding="utf-8")
    for h in ["## 元数据", "## 原始数据（逐次重复）", "## 统计", "## 公开资料（只记录数值事实）",
              "## 我方复测（第 7 章）", "## 映射的 /data 字段", "## 备注"]:
        assert h in txt
    assert "- 中位数：601 发/分" in txt and "置信度：B" in txt and "| r05 |" in txt
    with pytest.raises(record.RecordError):
        record.write_record(tmp_path, record.RecordSpec(id="X", name="x", reps=[record.Rep("r01", "shot.png")]))


def test_unmeasured_record_keeps_pending_markers(tmp_path):
    txt = record.render_record(record.RecordSpec(id="M-01", name="行走速度"))
    assert txt.count("待测") >= 8 and "置信度：—" in txt
    assert not re.search(r"\d+\.\d+ (米|发|秒)", txt)   # no invented numbers


def test_m0_catalogue_matches_doc_9_1():
    txt = DOC.read_text(encoding="utf-8")
    row = next(l for l in txt.splitlines() if l.startswith("| **批 M0**"))
    want = set()
    body = row.split("**步枪射击**")[1].split("| 让 M0")[0]
    for m in re.finditer(r"([WMH])-(\d\d)(?:[–-]\1?-?(\d\d))?", body):
        lo = int(m.group(2))
        hi = int(m.group(3)) if m.group(3) else lo
        want |= {f"{m.group(1)}-{n:02d}" for n in range(lo, hi + 1)}
    have = {it.id for it in M0_ITEMS}
    assert have == want, (sorted(have - want), sorted(want - have))


def test_catalogue_rows_unique_and_weapon_suffixed():
    rows = [r for _, r, _ in all_rows()]
    assert len(rows) == len(set(rows)) == 74
    assert "W-13.ger_kar98k" in rows and "W-13.us_m1_garand" not in rows
    assert "W-02.us_m1_garand" in rows and "M-01.rifleman" not in rows and "M-01" in rows


def test_own_fill_compares_with_7_3_tolerances(tmp_path):
    import json
    p = tmp_path / "summary.csv"
    record.write_summary(p, [])
    record.upsert_summary_row(p, _row(
        "H-01.us_m1_garand", median=2, min=2, max=2, n=10, uncertainty=0, unit="发"))
    own = tmp_path / "own.json"
    own.write_text(json.dumps({"entries": [
        {"id": "H-01.us_m1_garand", "unit": "发", "median": 2, "min": 2, "max": 2, "n": 1, "data_value": 2.0}]}))
    assert fill_own(p, own) == 1
    r = record.read_summary(p)[0]
    assert r["pass"] == "yes" and float(r["own_median"]) == 2.0
    own.write_text(json.dumps([{"id": "H-01.us_m1_garand", "unit": "发", "median": 3, "min": 3, "max": 3, "n": 1}]))
    fill_own(p, own)
    assert record.read_summary(p)[0]["pass"] == "no"      # shots to kill must be exactly equal
    own.write_text(json.dumps([{"id": "H-01.us_m1_garand", "unit": "秒", "median": 2}]))
    with pytest.raises(record.RecordError):
        fill_own(p, own)
    md = own_table_md([{"id": "A", "unit": "m", "median": None, "notes": "x|y"}])
    assert "不可测" in md and "x/y" in md
