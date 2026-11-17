"""Command line entry: `PYTHONPATH=tools/ref-analysis python3 -m ref_analysis <command> ...`.

All commands read private captures given on the command line (or under $REF_CAPTURE_DIR) and print numbers;
nothing is uploaded and nothing but numbers/text is written."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

import numpy as np

from . import audio_events, calibrate, extract_frames, frame_events, stats, suppression
from .common import probe
from .items import M0_ITEMS, all_rows
from .own_compare import fill_own, load_own, own_table_md
from .record import read_summary, validate_summary

ROOT = Path(__file__).resolve().parents[3]
MEAS = ROOT / "docs" / "measurements"


def _cap(p: str) -> str:
    """Relative capture paths resolve under $REF_CAPTURE_DIR."""
    q = Path(p)
    base = os.environ.get("REF_CAPTURE_DIR")
    return str(q if q.is_absolute() or not base else Path(base) / q)


def _roi(s: str | None):
    return tuple(int(x) for x in s.split(":")) if s else None


def cmd_probe(a):
    i = probe(_cap(a.video))
    print(json.dumps({"fps": float(i.fps), "avg_fps": float(i.avg_fps), "cfr": i.is_cfr, "w": i.width, "h": i.height,
                      "duration_s": i.duration_s, "nb_frames": i.nb_frames, "audio_rate": i.audio_rate,
                      "frame_unc_ms": i.frame_unc_s * 1000}, ensure_ascii=False))
    return 0 if i.is_cfr else 2


def cmd_audio_rpm(a):
    src = _cap(a.input)
    if not src.lower().endswith(".wav"):
        wav = Path(a.tmp) / (Path(src).stem + ".wav")
        extract_frames.extract_audio(src, wav)
        src = str(wav)
    r = audio_events.audio_rpm(src, expected_interval_s=a.interval, mode=a.mode, drop_first=a.drop_first)
    print(json.dumps({"n": r.n, "rpm": r.rpm, "rpm_unc": r.rpm_uncertainty, "first_s": r.first_s, "last_s": r.last_s,
                      "mean_interval_s": float(np.mean(r.intervals_s)) if len(r.intervals_s) else None,
                      "params": r.params}, ensure_ascii=False, default=float))
    return 0


def cmd_frame_rpm(a):
    info = probe(_cap(a.video))
    s = frame_events.luminance_series(_cap(a.video), roi=_roi(a.roi))
    on = frame_events.detect_onsets(s, level=a.level)
    fr = frame_events.rpm_from_frames(on, float(info.fps))
    print(json.dumps({"n": fr.n, "first": fr.first, "last": fr.last, "rpm": fr.rpm, "rpm_unc": fr.uncertainty_rpm,
                      "onsets": on if a.list else None}))
    return 0


def cmd_speed(a):
    r = calibrate.speed_from_marks(a.frame_a, a.frame_b, a.distance, a.fps, a.distance_unc)
    print(json.dumps({"speed_mps": r.speed_mps, "unc_mps": r.uncertainty_mps, "dt_s": r.dt_s}))
    return 0


def cmd_track(a):
    info = probe(_cap(a.video))
    roi = _roi(a.roi)
    if a.template:
        import cv2
        tpl = cv2.imread(a.template, cv2.IMREAD_GRAYSCALE)
        t = calibrate.track_template(_cap(a.video), tpl, roi=roi)
    else:
        t = calibrate.track_bright_blob(_cap(a.video), roi=roi, thresh=a.thresh)
    axis = 1 if a.axis == "x" else 2
    out = {"fps": float(info.fps), "frames": len(t)}
    if a.px_per_m:
        out["speed_mps"] = calibrate.fit_speed(t[:, 0], t[:, axis], float(info.fps), a.px_per_m)
    if a.csv:
        np.savetxt(a.csv, t, delimiter=",", header="frame,x_px,y_px", comments="", fmt=["%d", "%.4f", "%.4f"])
    print(json.dumps(out))
    return 0


def cmd_fov(a):
    xs = [float(v) for v in a.x.split(",")]
    th = [float(v) for v in a.theta.split(",")]
    f = calibrate.fov_from_pairs(xs, th, a.width)
    print(json.dumps(f.__dict__))
    return 0


def cmd_suppression(a):
    info = probe(_cap(a.video))
    fps = float(info.fps)
    sh = suppression.sharpness_curve(_cap(a.video))
    kp = suppression.sharp_keypoints(sh, (0, a.base_frames), fps, a.trigger_frame)
    res = {"sharp": kp.__dict__}
    if a.flow:
        fl = suppression.flow_shift_curve(_cap(a.video))
        res["shake"] = suppression.shake_metrics(fl, fps).__dict__
    if a.csv:
        np.savetxt(a.csv, np.c_[np.arange(len(sh)) / fps, sh], delimiter=",", header="t_s,laplacian_var", comments="")
    print(json.dumps(res, default=float))
    return 0


def cmd_stats(a):
    vals = [float(v) for v in a.values.split(",")]
    st = stats.summarize(vals, frame_unc=a.frame_unc)
    print(json.dumps({**st.__dict__, "confidence": stats.confidence(
        st.n, st.range_pct, public_conflict=a.public_conflict, cross_validated=a.cross_validated)}))
    return 0


def cmd_status(a):
    rows = {(r["id"]): r for r in read_summary(MEAS / "summary.csv")}
    missing, bad = [], []
    for it, rid, _ in all_rows():
        r = rows.get(rid)
        if r is None or r["status"] not in ("measured", "infeasible"):
            missing.append(rid)
        elif r["status"] == "measured" and r["pass"] != "yes":
            bad.append(rid)
        elif not (MEAS / f"{rid}.md").exists():
            missing.append(rid + " (record file)")
    total = len(all_rows())
    print(f"M0 batch: {total} rows; {total - len(missing)} measured/infeasible with record; "
          f"{len(missing)} missing; {len(bad)} measured but not pass")
    if a.verbose:
        print("missing:", ", ".join(missing))
    return 0 if not missing and not bad else 1


def cmd_validate(a):
    rows = validate_summary(MEAS / "summary.csv")
    print(f"summary.csv ok: {len(rows)} rows, 25 columns")
    return 0


def cmd_own_fill(a):
    n = fill_own(MEAS / "summary.csv", a.own)
    print(f"own-fill: {n} rows updated")
    return 0


def cmd_own_table(a):
    sys.stdout.write(own_table_md(load_own(a.own)))
    return 0


def cmd_checklist(a):
    print("| 行 ID | 项 | 场景 | 方法 | 重复 | 采集要点 |\n|---|---|---|---|---|---|")
    for it, rid, _ in all_rows():
        print(f"| {rid} | {it.item} | {it.scene} | {it.method} | >= {it.reps} | {it.protocol} |")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="ref_analysis", description=__doc__)
    sp = p.add_subparsers(dest="cmd", required=True)

    def add(name, fn, **kw):
        s = sp.add_parser(name, **kw)
        s.set_defaults(fn=fn)
        return s

    s = add("probe", cmd_probe, help="ffprobe: fps, CFR check (exit 2 when VFR -> discard capture)")
    s.add_argument("video")
    s = add("audio-rpm", cmd_audio_rpm, help="M-AUD: shot times and rpm from the audio track")
    s.add_argument("input", help="wav or video (relative paths resolve under $REF_CAPTURE_DIR)")
    s.add_argument("--tmp", default=os.environ.get("TMPDIR", "/tmp"))
    s.add_argument("--interval", type=float, default=None, help="expected shot interval in s")
    s.add_argument("--mode", choices=["onset", "peak"], default="onset")
    s.add_argument("--drop-first", type=int, default=0)
    s = add("frame-rpm", cmd_frame_rpm, help="M-FRM: flash onsets in a region -> rpm")
    s.add_argument("video")
    s.add_argument("--roi", help="W:H:X:Y of the muzzle-flash region")
    s.add_argument("--level", type=float, default=0.5)
    s.add_argument("--list", action="store_true")
    s = add("speed", cmd_speed, help="M-CAL-D: v = d / dt from two event frames")
    s.add_argument("--frame-a", type=int, required=True)
    s.add_argument("--frame-b", type=int, required=True)
    s.add_argument("--distance", type=float, required=True)
    s.add_argument("--distance-unc", type=float, default=0.0)
    s.add_argument("--fps", type=float, required=True)
    s = add("track", cmd_track, help="M-TPL: template / blob track -> csv, optional speed")
    s.add_argument("video")
    s.add_argument("--template")
    s.add_argument("--roi")
    s.add_argument("--thresh", type=int, default=128)
    s.add_argument("--axis", choices=["x", "y"], default="x")
    s.add_argument("--px-per-m", type=float)
    s.add_argument("--csv")
    s = add("fov", cmd_fov, help="M-ANG: fit horizontal FOV from (x_px, theta_deg) pairs")
    s.add_argument("--x", required=True)
    s.add_argument("--theta", required=True)
    s.add_argument("--width", type=int, default=1920)
    s = add("suppression", cmd_suppression, help="M-IMG / M-FLOW key points of one clip")
    s.add_argument("video")
    s.add_argument("--base-frames", type=int, required=True, help="frames [0, N) are the unsuppressed baseline")
    s.add_argument("--trigger-frame", type=int, required=True)
    s.add_argument("--flow", action="store_true")
    s.add_argument("--csv")
    s = add("stats", cmd_stats, help="median / range / uncertainty / confidence of comma separated values")
    s.add_argument("values")
    s.add_argument("--frame-unc", type=float, default=0.0)
    s.add_argument("--public-conflict", action="store_true")
    s.add_argument("--cross-validated", type=lambda v: v.lower() in ("1", "true", "yes"), default=None)
    s = add("status", cmd_status, help="M0 batch completeness gate (exit 1 while rows are missing)")
    s.add_argument("-v", "--verbose", action="store_true")
    add("validate", cmd_validate, help="validate docs/measurements/summary.csv (25 columns, enums)")
    s = add("own-fill", cmd_own_fill, help="fill own_* / pass columns from own_remeasure JSON")
    s.add_argument("own")
    s = add("own-table", cmd_own_table, help="markdown table of our own numbers")
    s.add_argument("own")
    add("checklist", cmd_checklist, help="markdown table of the M0 batch rows")
    return p


def main(argv=None) -> int:
    a = build_parser().parse_args(argv)
    return int(a.fn(a) or 0)
