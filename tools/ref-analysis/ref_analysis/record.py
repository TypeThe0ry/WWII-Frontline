"""Measurement record files and summary.csv (REFERENCE_MEASUREMENTS 8.1, 8.2, 8.3).

Rules: numbers and text only; the reference game is only ever called "参考作品"; unmeasured values stay
"待测"; summary.csv rows are upserted by (id, ref_version) so re-running is idempotent."""

from __future__ import annotations

import csv
import io
import math
from dataclasses import dataclass, field
from pathlib import Path

# 8.2, exactly 25 columns in this order.
SUMMARY_COLUMNS = [
    "id", "category", "item", "subject_id", "unit", "median", "min", "max", "n", "uncertainty",
    "fps", "method", "scene", "ref_version", "date", "confidence", "status", "public_source_agrees",
    "data_field", "data_value", "own_median", "own_error_pct", "tolerance_pct", "pass", "notes",
]
assert len(SUMMARY_COLUMNS) == 25

_FLOAT_COLS = {"median", "min", "max", "uncertainty", "data_value", "own_median", "own_error_pct", "tolerance_pct"}
_INT_COLS = {"n", "fps"}
_ENUMS = {
    "confidence": {"", "A", "B", "C", "D"},
    "status": {"measured", "stale", "pending", "infeasible"},
    "public_source_agrees": {"", "yes", "no", "na"},
    "pass": {"", "yes", "no", "na"},
}
FORBIDDEN_TEXT = ("\n", "\r")


class RecordError(ValueError):
    pass


def validate_row(row: dict) -> dict:
    """Check types and enums; returns a normalised dict with every column present as a string."""
    extra = set(row) - set(SUMMARY_COLUMNS)
    if extra:
        raise RecordError(f"unknown columns: {sorted(extra)}")
    out: dict[str, str] = {}
    for c in SUMMARY_COLUMNS:
        v = row.get(c, "")
        if v is None:
            v = ""
        if c in _FLOAT_COLS and v != "":
            f = float(v)
            if math.isnan(f) or math.isinf(f):
                raise RecordError(f"{c}: not finite")
            v = repr(round(f, 6))
        elif c in _INT_COLS and v != "":
            v = str(int(float(v)))
        else:
            v = str(v)
        if c in _ENUMS and v not in _ENUMS[c]:
            raise RecordError(f"{c}: {v!r} not in {sorted(_ENUMS[c])}")
        if any(ch in v for ch in FORBIDDEN_TEXT):
            raise RecordError(f"{c}: newline not allowed")
        out[c] = v
    if not out["id"]:
        raise RecordError("id is required")
    if out["status"] == "measured":
        for c in ("median", "min", "max", "n", "uncertainty", "confidence", "ref_version", "date"):
            if out[c] == "":
                raise RecordError(f"measured row needs {c}")
    return out


def read_summary(path: str | Path) -> list[dict]:
    p = Path(path)
    if not p.exists():
        return []
    with p.open(newline="", encoding="utf-8") as f:
        r = csv.DictReader(f)
        if r.fieldnames != SUMMARY_COLUMNS:
            raise RecordError(f"{path}: header mismatch: {r.fieldnames}")
        return [dict(x) for x in r]


def write_summary(path: str | Path, rows: list[dict]) -> None:
    buf = io.StringIO()
    w = csv.DictWriter(buf, fieldnames=SUMMARY_COLUMNS, lineterminator="\n")
    w.writeheader()
    for r in rows:
        w.writerow(r)
    Path(path).write_text(buf.getvalue(), encoding="utf-8")


def upsert_summary_row(path: str | Path, row: dict) -> bool:
    """Insert or replace the row with the same (id, ref_version). Returns True when the file changed."""
    new = validate_row(row)
    rows = read_summary(path)
    key = (new["id"], new["ref_version"])
    for i, r in enumerate(rows):
        if (r["id"], r["ref_version"]) == key:
            if r == new:
                return False
            rows[i] = new
            write_summary(path, rows)
            return True
    rows.append(new)
    write_summary(path, rows)
    return True


def validate_summary(path: str | Path) -> list[dict]:
    rows = read_summary(path)
    seen = set()
    for r in rows:
        validate_row(r)
        k = (r["id"], r["ref_version"])
        if k in seen:
            raise RecordError(f"duplicate row {k}")
        seen.add(k)
    return rows


# ------------------------------------------------------------------------------ record file (8.1)

@dataclass
class Rep:
    rep: str
    file: str            # private file name; never a path inside the repository
    start_frame: int | str = ""
    end_frame: int | str = ""
    events: int | str = ""
    value: float | str = ""
    unit: str = ""
    excluded: bool = False
    reason: str = ""


@dataclass
class RecordSpec:
    id: str
    name: str
    meta: dict = field(default_factory=dict)
    reps: list[Rep] = field(default_factory=list)
    stats: dict = field(default_factory=dict)       # n, median, min, max, uncertainty, unit, cross_validation, confidence
    public: list[str] = field(default_factory=list)
    own: list[dict] = field(default_factory=list)   # date, version, median, target, error_pct, tolerance, pass
    mapped: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)


def render_record(spec: RecordSpec) -> str:
    m = spec.meta
    pending = "待测"

    def g(k: str) -> str:
        v = m.get(k, "")
        return str(v) if v not in ("", None) else pending

    L = [f"# {spec.id} {spec.name}", "", "## 元数据"]
    L.append(f"- 测量项 ID：{spec.id}（对应 REFERENCE_MEASUREMENTS §4.x）")
    L.append(f"- 参考作品版本/构建号：{g('version')}")
    L.append(f"- 测量日期：{g('date')}")
    L.append(f"- 测量者：{g('measurer')}   分析者：{g('analyst')}")
    L.append(f"- 录制软件与版本：{g('recorder')}")
    L.append(f"- 采集帧率 / 分辨率 / 编码码率：{g('capture')}")
    L.append(f"- 游戏内 FOV 设置：{g('fov')}   鼠标 DPI×灵敏度：{g('mouse')}")
    L.append(f"- 帧时间不确定度：±1 帧 = ±{g('frame_unc')}")
    L.append(f"- 场景编号：{g('scene')}   场景描述（站位、地图、天气、时间）：{g('scene_desc')}")
    L.append(f"- 武器/载具/兵种 ID：{g('subject')}")
    L.append(f"- 分析方法码：{g('method')}")
    L.append(f"- 分析脚本版本：{g('script')}")
    L += ["", "## 原始数据（逐次重复）",
          "| 重复 | 录像文件名（私有存储，不入库） | 起始帧 | 结束帧 | 事件数 N | 原始值 | 单位 | 是否剔除 | 剔除原因 |",
          "|---|---|---|---|---|---|---|---|---|"]
    if spec.reps:
        for r in spec.reps:
            val = f"{r.value:.6g}" if isinstance(r.value, (int, float)) else r.value
            L.append(f"| {r.rep} | {r.file} | {r.start_frame} | {r.end_frame} | {r.events} | {val} | {r.unit} | "
                     f"{'是' if r.excluded else '否'} | {r.reason} |")
    else:
        L.append(f"| — | {pending} | | | | {pending} | | | |")
    s = spec.stats
    u = s.get("unit", "")
    L += ["", "## 统计"]
    if s:
        L.append(f"- 样本数 n：{s['n']}")
        L.append(f"- 中位数：{s['median']:.6g} {u}")
        L.append(f"- 极差 [最小, 最大]：{s['max'] - s['min']:.6g} [{s['min']:.6g}, {s['max']:.6g}]")
        L.append(f"- 不确定度（max(极差/2, 1 帧)）：±{s['uncertainty']:.6g} {u}")
        L.append(f"- 交叉验证（第二种方法/公开资料）：{s.get('cross_validation', pending)}")
        L.append(f"- 置信度：{s.get('confidence', pending)}")
    else:
        L += [f"- 样本数 n：{pending}", f"- 中位数：{pending}", f"- 极差 [最小, 最大]：{pending}",
              f"- 不确定度（max(极差/2, 1 帧)）：{pending}", f"- 交叉验证（第二种方法/公开资料）：{pending}",
              "- 置信度：—"]
    L += ["", "## 公开资料（只记录数值事实）"]
    L += [f"- {x}" for x in spec.public] or ["- （无）"]
    L += ["", "## 我方复测（第 7 章）",
          "| 日期 | 我方版本（git 提交） | 复测中位数 | 目标值 | 误差 % | 容限 | 通过 |",
          "|---|---|---|---|---|---|---|"]
    for o in spec.own:
        L.append(f"| {o.get('date', '')} | {o.get('version', '')} | {o.get('median', '')} | {o.get('target', '')} | "
                 f"{o.get('error_pct', '')} | {o.get('tolerance', '')} | {o.get('pass', '')} |")
    L += ["", "## 映射的 /data 字段"]
    L += [f"- {x}" for x in spec.mapped] or ["- （未映射）"]
    L += ["", "## 备注"]
    L += [f"- {x}" for x in spec.notes] or ["- （无）"]
    return "\n".join(L) + "\n"


def write_record(directory: str | Path, spec: RecordSpec) -> Path:
    """Writes `{directory}/{ID}.md`, refusing anything that looks like an image/video/audio path."""
    p = Path(directory) / f"{spec.id}.md"
    for r in spec.reps:
        if Path(r.file).suffix.lower() in {".png", ".jpg", ".jpeg", ".gif", ".bmp"}:
            raise RecordError("records may only name private capture files, not images")
    p.write_text(render_record(spec), encoding="utf-8")
    return p


def row_from_stats(item, row_id: str, subject: str, st, *, fps: int | None, method: str, scene: str,
                   ref_version: str, date: str, confidence: str, public: str = "na",
                   data_field: str = "", data_value=None, notes: str = "", status: str = "measured") -> dict:
    return {
        "id": row_id, "category": item.category, "item": item.item, "subject_id": subject,
        "unit": item.unit, "median": st.median, "min": st.min, "max": st.max, "n": st.n,
        "uncertainty": st.uncertainty, "fps": fps if fps is not None else "", "method": method,
        "scene": scene, "ref_version": ref_version, "date": date, "confidence": confidence,
        "status": status, "public_source_agrees": public, "data_field": data_field or item.data_field,
        "data_value": "" if data_value is None else data_value, "notes": notes,
    }
